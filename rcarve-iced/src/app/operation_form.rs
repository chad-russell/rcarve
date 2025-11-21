use rcarve::ids::CurveId;
use rcarve::{CutSide, Operation, OperationTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKindForm {
    Pocket,
    Profile,
    VCarve,
}

#[derive(Debug, Clone)]
pub struct OperationForm {
    pub kind: OperationKindForm,
    pub depth: String,
    pub cut_side: CutSide,
    pub tool_index: Option<usize>,
    pub clearance_tool_index: Option<usize>,
    pub vcarve_max_depth: String,
    pub selection_snapshot: Vec<CurveId>,
    pub error: Option<String>,
}

impl Default for OperationForm {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationForm {
    pub fn new() -> Self {
        Self {
            kind: OperationKindForm::Profile,
            depth: String::from("1.0"),
            cut_side: CutSide::Outside,
            tool_index: None,
            clearance_tool_index: None,
            vcarve_max_depth: String::new(),
            selection_snapshot: Vec::new(),
            error: None,
        }
    }

    pub fn from_operation(operation: &Operation) -> Self {
        match operation {
            Operation::Profile {
                target_depth,
                cut_side,
                tool_index,
                targets,
            } => Self {
                kind: OperationKindForm::Profile,
                depth: target_depth.to_string(),
                cut_side: cut_side.clone(),
                tool_index: Some(*tool_index),
                clearance_tool_index: None,
                vcarve_max_depth: String::new(),
                selection_snapshot: curves_from_target(targets),
                error: None,
            },
            Operation::Pocket {
                target_depth,
                tool_index,
                target,
            } => Self {
                kind: OperationKindForm::Pocket,
                depth: target_depth.to_string(),
                cut_side: CutSide::Inside,
                tool_index: Some(*tool_index),
                clearance_tool_index: None,
                vcarve_max_depth: String::new(),
                selection_snapshot: curves_from_target(target),
                error: None,
            },
            Operation::VCarve {
                target_depth,
                tool_index,
                targets,
                clearance_tool_index,
            } => Self {
                kind: OperationKindForm::VCarve,
                depth: String::new(),
                cut_side: CutSide::OnLine,
                tool_index: Some(*tool_index),
                clearance_tool_index: *clearance_tool_index,
                vcarve_max_depth: target_depth
                    .map(|depth| depth.to_string())
                    .unwrap_or_default(),
                selection_snapshot: curves_from_target(targets),
                error: None,
            },
        }
    }

    pub fn update_selection(&mut self, curves: &[CurveId]) {
        self.selection_snapshot = curves.to_vec();
    }

    pub fn validate(
        &mut self,
        selected_curves: &[CurveId],
        tool_count: usize,
    ) -> Result<Operation, String> {
        self.error = None;

        if selected_curves.is_empty() {
            self.error = Some("Select at least one curve in the canvas before saving.".to_string());
            return Err(self.error.clone().unwrap());
        }

        self.selection_snapshot = selected_curves.to_vec();

        let depth_value = if self.kind == OperationKindForm::VCarve {
            parse_optional_positive(&self.vcarve_max_depth, "Max depth")?
        } else {
            Some(parse_positive(&self.depth, "Depth")?)
        };

        let tool_index = match self.tool_index {
            Some(index) if index < tool_count => index,
            Some(_) => {
                let error = "Selected tool is no longer available.".to_string();
                self.error = Some(error.clone());
                return Err(error);
            }
            None => {
                let error = "Choose a tool before saving.".to_string();
                self.error = Some(error.clone());
                return Err(error);
            }
        };

        let target = OperationTarget::Curves(selected_curves.to_vec());

        let operation = match self.kind {
            OperationKindForm::Profile => Operation::Profile {
                target_depth: depth_value.expect("profile depth set"),
                cut_side: self.cut_side.clone(),
                tool_index,
                targets: target,
            },
            OperationKindForm::Pocket => Operation::Pocket {
                target_depth: depth_value.expect("pocket depth set"),
                tool_index,
                target,
            },
            OperationKindForm::VCarve => Operation::VCarve {
                target_depth: depth_value,
                tool_index,
                targets: target,
                clearance_tool_index: self
                    .clearance_tool_index
                    .filter(|index| *index < tool_count),
            },
        };

        Ok(operation)
    }
}

fn curves_from_target(target: &OperationTarget) -> Vec<CurveId> {
    match target {
        OperationTarget::Curves(curves) => curves.clone(),
        OperationTarget::Region(_) => Vec::new(),
    }
}

fn parse_positive(value: &str, label: &str) -> Result<f64, String> {
    let parsed: f64 = value
        .trim()
        .parse()
        .map_err(|_| format!("{label} must be a number"))?;
    if parsed <= 0.0 {
        return Err(format!("{label} must be greater than zero"));
    }
    Ok(parsed)
}

fn parse_optional_positive(value: &str, label: &str) -> Result<Option<f64>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    parse_positive(trimmed, label).map(Some)
}
