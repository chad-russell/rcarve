use rcarve::{Tool, ToolType};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Endmill,
    VBit,
    Ballnose,
}

impl ToolKind {
    pub fn from_tool_type(tool_type: &ToolType) -> Self {
        match tool_type {
            ToolType::Endmill { .. } => ToolKind::Endmill,
            ToolType::VBit { .. } => ToolKind::VBit,
            ToolType::Ballnose { .. } => ToolKind::Ballnose,
        }
    }
}

impl fmt::Display for ToolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolKind::Endmill => write!(f, "Endmill"),
            ToolKind::VBit => write!(f, "V-bit"),
            ToolKind::Ballnose => write!(f, "Ballnose"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolForm {
    pub name: String,
    pub diameter: String,
    pub stepover: String,
    pub pass_depth: String,
    pub vbit_angle: String,
    pub kind: ToolKind,
    pub name_error: Option<String>,
    pub diameter_error: Option<String>,
    pub stepover_error: Option<String>,
    pub pass_depth_error: Option<String>,
    pub vbit_angle_error: Option<String>,
}

impl Default for ToolForm {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolForm {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            diameter: String::new(),
            stepover: "0.4".to_string(),
            pass_depth: String::new(),
            vbit_angle: "60".to_string(),
            kind: ToolKind::Endmill,
            name_error: None,
            diameter_error: None,
            stepover_error: None,
            pass_depth_error: None,
            vbit_angle_error: None,
        }
    }

    pub fn from_tool(tool: &Tool) -> Self {
        let mut form = Self {
            name: tool.name.clone(),
            diameter: format!("{}", tool.diameter),
            stepover: format!("{}", tool.stepover),
            pass_depth: format!("{}", tool.pass_depth),
            vbit_angle: "60".to_string(),
            kind: ToolKind::from_tool_type(&tool.tool_type),
            name_error: None,
            diameter_error: None,
            stepover_error: None,
            pass_depth_error: None,
            vbit_angle_error: None,
        };

        if let ToolType::VBit { angle_degrees } = &tool.tool_type {
            form.vbit_angle = format!("{}", angle_degrees);
        }

        form
    }

    pub fn validate(&mut self) -> Result<Tool, String> {
        self.clear_errors();
        let mut errors = Vec::new();

        if self.name.trim().is_empty() {
            self.name_error = Some("Name is required".to_string());
            errors.push("Name is required".to_string());
        }

        let diameter = match self.parse_positive(&self.diameter, "Diameter") {
            Ok(value) => value,
            Err(err) => {
                self.diameter_error = Some(err.clone());
                errors.push(err);
                0.0
            }
        };

        let stepover = match self.parse_fraction(&self.stepover, "Stepover") {
            Ok(value) => value,
            Err(err) => {
                self.stepover_error = Some(err.clone());
                errors.push(err);
                0.0
            }
        };

        let pass_depth = match self.parse_positive(&self.pass_depth, "Pass depth") {
            Ok(value) => value,
            Err(err) => {
                self.pass_depth_error = Some(err.clone());
                errors.push(err);
                0.0
            }
        };

        let tool_type = match self.kind {
            ToolKind::Endmill => ToolType::Endmill { diameter },
            ToolKind::Ballnose => ToolType::Ballnose { diameter },
            ToolKind::VBit => match self.parse_positive(&self.vbit_angle, "V-bit angle") {
                Ok(angle) => ToolType::VBit {
                    angle_degrees: angle,
                },
                Err(err) => {
                    self.vbit_angle_error = Some(err.clone());
                    errors.push(err);
                    ToolType::VBit { angle_degrees: 0.0 }
                }
            },
        };

        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }

        Ok(Tool {
            name: self.name.trim().to_string(),
            diameter,
            stepover,
            pass_depth,
            tool_type,
        })
    }

    pub fn set_kind(&mut self, kind: ToolKind) {
        self.kind = kind;
        if matches!(kind, ToolKind::VBit) && self.vbit_angle.trim().is_empty() {
            self.vbit_angle = "60".to_string();
        }
    }

    fn parse_positive(&self, input: &str, label: &str) -> Result<f64, String> {
        let value: f64 = input
            .trim()
            .parse()
            .map_err(|_| format!("{label} must be a number"))?;
        if value <= 0.0 {
            return Err(format!("{label} must be greater than zero"));
        }
        Ok(value)
    }

    fn parse_fraction(&self, input: &str, label: &str) -> Result<f64, String> {
        let value: f64 = input
            .trim()
            .parse()
            .map_err(|_| format!("{label} must be a number"))?;
        if !(0.0..=1.0).contains(&value) {
            return Err(format!("{label} must be between 0.0 and 1.0"));
        }
        Ok(value)
    }

    fn clear_errors(&mut self) {
        self.name_error = None;
        self.diameter_error = None;
        self.stepover_error = None;
        self.pass_depth_error = None;
        self.vbit_angle_error = None;
    }
}
