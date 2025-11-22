use crate::geometry::offset::offset_polygon;
use crate::geometry::{CurveId, Region};
use crate::types::{Tool, ToolType};
use crate::{
    generate_pocket_toolpath, generate_profile_toolpath, generate_vcarve_toolpath, CarvePolygon,
    Operation, OperationTarget, Project, ToolLibrary, Toolpath, ToolpathArtifact, ToolpathPass,
    ToolpathPassKind, ToolpathStatus,
};
use anyhow::{anyhow, Context, Result};
use kurbo::Affine;
use std::collections::HashMap;

const FLATTEN_TOLERANCE: f64 = 0.25;

#[derive(Debug, Clone)]
pub struct ToolpathGenerationReport {
    pub operation_index: usize,
    pub status: ToolpathStatus,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

/// Generate toolpaths for every operation in the project, updating cached artifacts.
pub fn generate_toolpaths_for_operations(
    project: &mut Project,
    tools: &ToolLibrary,
) -> Vec<ToolpathGenerationReport> {
    let mut reports = Vec::new();

    for index in 0..project.operations.len() {
        let operation = project.operations[index].clone();
        match generate_toolpath_for_operation(project, tools, index, &operation) {
            Ok((artifact, warnings)) => {
                let status = ToolpathStatus::Ready {
                    generated_at_epoch_ms: artifact.generated_at_epoch_ms,
                    warning_count: warnings.len(),
                };
                if let Err(err) = project.attach_toolpath(index, artifact) {
                    reports.push(ToolpathGenerationReport {
                        operation_index: index,
                        status: ToolpathStatus::Dirty,
                        warnings: vec![],
                        error: Some(err.to_string()),
                    });
                } else {
                    reports.push(ToolpathGenerationReport {
                        operation_index: index,
                        status,
                        warnings,
                        error: None,
                    });
                }
            }
            Err(err) => {
                reports.push(ToolpathGenerationReport {
                    operation_index: index,
                    status: ToolpathStatus::Dirty,
                    warnings: vec![],
                    error: Some(err.to_string()),
                });
            }
        }
    }

    reports
}

fn generate_toolpath_for_operation(
    project: &mut Project,
    tools: &ToolLibrary,
    operation_index: usize,
    operation: &Operation,
) -> Result<(ToolpathArtifact, Vec<String>)> {
    let mut warnings = Vec::new();
    let shapes = &project.shapes;
    
    // Build a map of curve_id -> transform from imports
    let curve_transforms = build_curve_transform_map(project);

    match operation {
        Operation::Profile {
            target_depth,
            cut_side,
            tool_index,
            targets,
        } => {
            let tool = tools
                .tools
                .get(*tool_index)
                .ok_or_else(|| anyhow!("Tool #{tool_index} not found"))?;

            let polygon = first_polygon(shapes, targets, &curve_transforms)?.context("Profile requires geometry")?;
            let toolpath = generate_profile_toolpath(&polygon, tool, cut_side, *target_depth)
                .with_context(|| {
                    format!("Profile operation {operation_index} failed to generate")
                })?;
            let pass = ToolpathPass::new(*tool_index, ToolpathPassKind::Finish, toolpath.clone());

            Ok((
                ToolpathArtifact {
                    operation_index,
                    toolpath,
                    passes: vec![pass],
                    generated_at_epoch_ms: current_epoch_ms(),
                    warnings: warnings.clone(),
                    is_valid: true,
                },
                warnings,
            ))
        }
        Operation::Pocket {
            target_depth,
            tool_index,
            target,
        } => {
            let tool = tools
                .tools
                .get(*tool_index)
                .ok_or_else(|| anyhow!("Tool #{tool_index} not found"))?;

            let (outer, holes) = match target {
                OperationTarget::Region(region_id) => {
                    let region = shapes
                        .get_region(region_id)
                        .ok_or_else(|| anyhow!("Region {:?} not found", region_id))?;
                    flatten_region(shapes, region, &curve_transforms)?
                }
                OperationTarget::Curves(curves) => {
                    let mut polys = flatten_curves(shapes, curves, &curve_transforms)?;
                    let outer = polys
                        .pop()
                        .ok_or_else(|| anyhow!("Pocket requires at least one closed curve"))?;
                    (outer, polys)
                }
            };

            let toolpath = generate_pocket_toolpath(&outer, &holes, tool, *target_depth)
                .with_context(|| format!("Pocket operation {operation_index} failed"))?;
            let pass = ToolpathPass::new(*tool_index, ToolpathPassKind::Finish, toolpath.clone());

            Ok((
                ToolpathArtifact {
                    operation_index,
                    toolpath,
                    passes: vec![pass],
                    generated_at_epoch_ms: current_epoch_ms(),
                    warnings: warnings.clone(),
                    is_valid: true,
                },
                warnings,
            ))
        }
        Operation::VCarve {
            target_depth,
            tool_index,
            targets,
            clearance_tool_index,
        } => {
            let tool = tools
                .tools
                .get(*tool_index)
                .ok_or_else(|| anyhow!("Tool #{tool_index} not found"))?;

            let carve_polygons = collect_vcarve_polygons(shapes, targets, &curve_transforms)?;

            let mut passes = Vec::new();

            if let Some(clearance_index) = clearance_tool_index {
                let clearance_tool = tools
                    .tools
                    .get(*clearance_index)
                    .ok_or_else(|| anyhow!("Tool #{clearance_index} not found"))?;

                let clearance_depth = match target_depth {
                    Some(depth) if *depth > 0.0 => *depth,
                    _ => {
                        warnings.push(
                            "Clearance tool selected without a max depth; defaulting to 1mm."
                                .to_string(),
                        );
                        1.0
                    }
                };

                let angle_deg = match tool.tool_type {
                    ToolType::VBit { angle_degrees } => angle_degrees,
                    _ => return Err(anyhow!("V-carve operation requires a V-bit tool")),
                };
                let rad = angle_deg.to_radians() / 2.0;
                let limit_dist = clearance_depth * rad.tan();

                let inner_polygons = offset_polygon(&carve_polygons, limit_dist)?;

                let clearance_toolpath =
                    generate_clearance_toolpath(&inner_polygons, clearance_tool, clearance_depth)
                        .with_context(|| {
                            format!("Clearance toolpath for operation {operation_index} failed")
                        })?;

                passes.push(ToolpathPass::new(
                    *clearance_index,
                    ToolpathPassKind::Clearance,
                    clearance_toolpath.clone(),
                ));
            }

            let finish_toolpath = generate_vcarve_toolpath(&carve_polygons, tool, *target_depth)
                .with_context(|| format!("V-carve operation {operation_index} failed"))?;
            passes.push(ToolpathPass::new(
                *tool_index,
                ToolpathPassKind::Finish,
                finish_toolpath.clone(),
            ));

            Ok((
                ToolpathArtifact {
                    operation_index,
                    toolpath: finish_toolpath,
                    passes,
                    generated_at_epoch_ms: current_epoch_ms(),
                    warnings: warnings.clone(),
                    is_valid: true,
                },
                warnings,
            ))
        }
    }
}

pub fn polygons_for_operation(project: &Project, index: usize) -> Result<Vec<Vec<(f64, f64)>>> {
    let shapes = &project.shapes;
    let curve_transforms = build_curve_transform_map(project);
    let operation = project
        .operations
        .get(index)
        .ok_or_else(|| anyhow!("invalid operation index {index}"))?;

    match operation {
        Operation::Profile { targets, .. } => {
            let polygon = first_polygon(shapes, targets, &curve_transforms)?;
            Ok(polygon.into_iter().collect())
        }
        Operation::Pocket { target, .. } => {
            let (mut outer, mut holes) = match target {
                OperationTarget::Region(region_id) => {
                    let region = shapes
                        .get_region(region_id)
                        .ok_or_else(|| anyhow!("Region {:?} not found", region_id))?;
                    flatten_region(shapes, region, &curve_transforms)?
                }
                OperationTarget::Curves(curves) => {
                    let mut polys = flatten_curves(shapes, curves, &curve_transforms)?;
                    let outer = polys
                        .pop()
                        .ok_or_else(|| anyhow!("Pocket requires at least one closed curve"))?;
                    (outer, polys)
                }
            };
            close_loop(&mut outer);
            for hole in &mut holes {
                close_loop(hole);
            }
            let mut polygons = vec![outer];
            polygons.extend(holes);
            Ok(polygons)
        }
        Operation::VCarve { targets, .. } => match targets {
            OperationTarget::Curves(curves) => flatten_curves(shapes, curves, &curve_transforms),
            OperationTarget::Region(region_id) => {
                let region = shapes
                    .get_region(region_id)
                    .ok_or_else(|| anyhow!("Region {:?} not found", region_id))?;
                let (mut outer, mut holes) = flatten_region(shapes, region, &curve_transforms)?;
                close_loop(&mut outer);
                for hole in &mut holes {
                    close_loop(hole);
                }
                let mut polygons = vec![outer];
                polygons.extend(holes);
                Ok(polygons)
            }
        },
    }
}

/// Build a map from curve IDs to their transforms from imports
fn build_curve_transform_map(project: &Project) -> HashMap<CurveId, Affine> {
    let mut map = HashMap::new();
    for import in &project.imported_svgs {
        for &curve_id in &import.curve_ids {
            map.insert(curve_id, import.transform);
        }
    }
    map
}

fn first_polygon(
    shapes: &crate::geometry::ShapeRegistry,
    target: &OperationTarget,
    curve_transforms: &HashMap<CurveId, Affine>,
) -> Result<Option<Vec<(f64, f64)>>> {
    match target {
        OperationTarget::Curves(curves) => {
            if let Some(first) = curves.first() {
                let mut points = flatten_curve(shapes, first, curve_transforms)?;
                close_loop(&mut points);
                Ok(Some(points))
            } else {
                Ok(None)
            }
        }
        OperationTarget::Region(region_id) => {
            let region = shapes
                .get_region(region_id)
                .ok_or_else(|| anyhow!("Region {:?} not found", region_id))?;
            let (mut outer, _) = flatten_region(shapes, region, curve_transforms)?;
            close_loop(&mut outer);
            Ok(Some(outer))
        }
    }
}

fn flatten_curves(
    shapes: &crate::geometry::ShapeRegistry,
    curves: &[CurveId],
    curve_transforms: &HashMap<CurveId, Affine>,
) -> Result<Vec<Vec<(f64, f64)>>> {
    let mut result = Vec::new();
    for id in curves {
        let mut points = flatten_curve(shapes, id, curve_transforms)?;
        close_loop(&mut points);
        if points.len() >= 3 {
            result.push(points);
        }
    }
    Ok(result)
}

fn flatten_region(
    shapes: &crate::geometry::ShapeRegistry,
    region: &Region,
    curve_transforms: &HashMap<CurveId, Affine>,
) -> Result<(Vec<(f64, f64)>, Vec<Vec<(f64, f64)>>)> {
    let mut outer = flatten_curve(shapes, &region.outer, curve_transforms)?;
    close_loop(&mut outer);

    let mut holes = Vec::new();
    for hole in &region.holes {
        let mut points = flatten_curve(shapes, hole, curve_transforms)?;
        close_loop(&mut points);
        if points.len() >= 3 {
            holes.push(points);
        }
    }

    Ok((outer, holes))
}

fn collect_vcarve_polygons(
    shapes: &crate::geometry::ShapeRegistry,
    targets: &OperationTarget,
    curve_transforms: &HashMap<CurveId, Affine>,
) -> Result<Vec<CarvePolygon>> {
    let mut polygons = Vec::new();
    match targets {
        OperationTarget::Curves(curves) => {
            let loops = flatten_curves(shapes, curves, curve_transforms)?;
            for outer in loops {
                polygons.push(CarvePolygon {
                    outer,
                    holes: Vec::new(),
                });
            }
        }
        OperationTarget::Region(region_id) => {
            let region = shapes
                .get_region(region_id)
                .ok_or_else(|| anyhow!("Region {:?} not found", region_id))?;
            let (outer, holes) = flatten_region(shapes, region, curve_transforms)?;
            polygons.push(CarvePolygon { outer, holes });
        }
    }

    if polygons.is_empty() {
        Err(anyhow!("V-carve requires at least one closed polygon"))
    } else {
        Ok(polygons)
    }
}

fn generate_clearance_toolpath(
    polygons: &[CarvePolygon],
    tool: &Tool,
    depth: f64,
) -> Result<Toolpath> {
    if depth <= 0.0 {
        return Err(anyhow!("Clearance depth must be positive"));
    }

    let mut paths = Vec::new();
    for poly in polygons {
        let toolpath = generate_pocket_toolpath(&poly.outer, &poly.holes, tool, depth)?;
        paths.extend(toolpath.paths);
    }

    Ok(Toolpath { paths })
}

fn flatten_curve(
    shapes: &crate::geometry::ShapeRegistry,
    id: &CurveId,
    curve_transforms: &HashMap<CurveId, Affine>,
) -> Result<Vec<(f64, f64)>> {
    let curve = shapes
        .get_curve(id)
        .ok_or_else(|| anyhow!("Curve {:?} not found", id))?;
    
    // Apply transform if this curve belongs to an import
    let mut curve = curve.clone();
    if let Some(&transform) = curve_transforms.get(id) {
        curve.apply_affine(transform);
    }
    
    let points = curve.flatten(FLATTEN_TOLERANCE);
    if points.is_empty() {
        return Err(anyhow!("Curve {:?} produced no points", id));
    }
    Ok(points)
}

fn close_loop(points: &mut Vec<(f64, f64)>) {
    if let (Some(first), Some(last)) = (points.first().cloned(), points.last().cloned()) {
        if (first.0 - last.0).abs() > f64::EPSILON || (first.1 - last.1).abs() > f64::EPSILON {
            points.push(first);
        }
    }
}

fn current_epoch_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_millis() as u64)
        .unwrap_or_default()
}
