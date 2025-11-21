use iced::widget::{center, container, mouse_area, opaque, stack};
use iced::{Color, Element};

use super::Message;

pub fn format_dimension(value: f64) -> String {
    let formatted = format!("{value:.3}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');

    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn parse_dimension(input: &str, label: &str) -> Result<f64, String> {
    input
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("{label} must be a number"))
}

pub fn parse_origin(input: &str) -> Result<Option<(f64, f64, f64)>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let parts: Vec<_> = trimmed
        .split(|c| c == ',' || c == ' ')
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() != 3 {
        return Err("Origin must have three values separated by commas.".to_string());
    }

    let parse_part = |part: &str, label: &str| -> Result<f64, String> {
        part.parse::<f64>()
            .map_err(|_| format!("Origin {label} must be a number"))
    };

    let x = parse_part(parts[0], "X")?;
    let y = parse_part(parts[1], "Y")?;
    let z = parse_part(parts[2], "Z")?;

    Ok(Some((x, y, z)))
}

pub fn format_origin_label(origin: Option<(f64, f64, f64)>) -> String {
    origin
        .map(|(x, y, z)| {
            format!(
                "({}, {}, {})",
                format_dimension(x),
                format_dimension(y),
                format_dimension(z)
            )
        })
        .unwrap_or_else(|| "—".to_string())
}

pub fn modal_overlay<'a>(
    base: Element<'a, Message>,
    content: Element<'a, Message>,
    on_blur: Message,
) -> Element<'a, Message> {
    stack![
        base,
        opaque(
            mouse_area(center(opaque(content)).style(|_theme| {
                container::Style {
                    background: Some(
                        Color {
                            a: 0.7,
                            ..Color::BLACK
                        }
                        .into(),
                    ),
                    ..container::Style::default()
                }
            }))
            .on_press(on_blur)
        )
    ]
    .into()
}
