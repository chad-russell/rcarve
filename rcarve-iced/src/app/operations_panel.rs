use iced::border::Border;
use iced::widget::{button, column, container, mouse_area, row, text};
use iced::{Alignment, Color, Element, Length};
use rcarve::{OperationKind, OperationSummary, ToolLibrary, ToolpathStatus};
use std::collections::HashSet;

use super::{Message, canvas_view::toolpath_color};

const PANEL_BG: Color = Color::from_rgb(0.18, 0.18, 0.2);
const CARD_BG: Color = Color::from_rgb(0.14, 0.14, 0.16);
const MUTED_TEXT: Color = Color::from_rgb(0.72, 0.75, 0.82);

pub fn operations_view(
    entries: Vec<(OperationSummary, ToolpathStatus)>,
    tools: &ToolLibrary,
    visible_paths: &HashSet<usize>,
    is_generating: bool,
    show_debug_polygons: bool,
) -> Element<'static, Message> {
    let header = operations_header(is_generating, show_debug_polygons);

    let body: Element<'static, Message> = if entries.is_empty() {
        container(
            column![
                text("No operations yet.")
                    .size(16)
                    .style(|_| text_style(MUTED_TEXT)),
                text("Use “Add Operation” to create your first toolpath.")
                    .size(12)
                    .style(|_| text_style(MUTED_TEXT)),
            ]
            .spacing(6)
            .align_x(Alignment::Center),
        )
        .padding(32)
        .style(|_| card_style())
        .into()
    } else {
        column(
            entries
                .iter()
                .map(|(summary, status)| {
                    operation_card(
                        summary,
                        status,
                        tools,
                        visible_paths.contains(&summary.index),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .spacing(16)
        .into()
    };

    let mut layout = column![header, body].spacing(18);

    if !entries.is_empty() {
        layout = layout.push(
            text("Color swatches match the canvas overlay. Hover a card to highlight its path.")
                .size(12)
                .style(|_| text_style(MUTED_TEXT)),
        );
    }

    container(layout)
        .padding(24)
        .width(Length::Fill)
        .style(|_| panel_style())
        .into()
}

fn operations_header(is_generating: bool, show_debug_polygons: bool) -> Element<'static, Message> {
    let mut generate_button = button(if is_generating {
        "Generating..."
    } else {
        "Generate Toolpaths"
    })
    .padding([10, 18]);

    if !is_generating {
        generate_button = generate_button.on_press(Message::GenerateToolpaths);
    }

    column![
        text("Operations")
            .size(26)
            .style(|_| text_style(Color::WHITE)),
        text("Sequence your CAM steps and keep toolpaths in sync.")
            .size(14)
            .style(|_| text_style(MUTED_TEXT)),
        button("+ Add Operation")
            .on_press(Message::AddOperation)
            .padding([10, 18]),
        generate_button,
        button(if show_debug_polygons {
            "Hide Polygon Outlines"
        } else {
            "Show Polygon Outlines"
        })
        .on_press(Message::ToggleDebugPolygons)
        .padding([10, 18]),
    ]
    .spacing(16)
    .into()
}

fn operation_card(
    summary: &OperationSummary,
    status: &ToolpathStatus,
    tools: &ToolLibrary,
    is_visible: bool,
) -> Element<'static, Message> {
    let kind_label = match summary.kind {
        OperationKind::Profile => "Profile",
        OperationKind::Pocket => "Pocket",
        OperationKind::VCarve => "V-Carve",
    };

    let tool_label = tools
        .tools
        .get(summary.primary_tool_index)
        .map(|tool| tool.name.clone())
        .unwrap_or_else(|| format!("Tool #{}", summary.primary_tool_index + 1));

    let mut details = column![
        info_row("Type", kind_label.to_string()),
        info_row(
            "Targets",
            format!(
                "{} {}",
                summary.target_count,
                if summary.target_count == 1 {
                    "curve"
                } else {
                    "curves"
                }
            )
        ),
        info_row("Tool", tool_label.clone()),
    ]
    .spacing(4)
    .width(Length::Fill);

    if let Some(clearance) = summary.clearance_tool_index {
        let label = tools
            .tools
            .get(clearance)
            .map(|tool| tool.name.clone())
            .unwrap_or_else(|| format!("Tool #{}", clearance + 1));
        details = details.push(info_row("Clearance", label));
    }

    let actions = column![
        button("Edit")
            .padding([6, 14])
            .on_press(Message::EditOperation(summary.index)),
        button("Delete")
            .padding([6, 14])
            .on_press(Message::DeleteOperation(summary.index)),
        button("Clear Path")
            .padding([6, 14])
            .on_press(Message::ClearToolpath(summary.index)),
        button("Log Poly")
            .padding([6, 14])
            .on_press(Message::LogOperationPolygons(summary.index)),
    ]
    .spacing(10);

    let visibility_control: Element<'static, Message> = match status {
        ToolpathStatus::Ready { .. } => button(if is_visible { "Hide Path" } else { "Show Path" })
            .padding([6, 14])
            .on_press(Message::ToggleToolpathVisibility(summary.index))
            .into(),
        _ => text("Generate toolpath to view")
            .size(12)
            .style(|_| text_style(MUTED_TEXT))
            .into(),
    };

    let swatch = color_swatch(toolpath_color(summary.index));

    mouse_area(
        container(
            column![
                swatch,
                status_badge(status),
                text(kind_label)
                    .size(22)
                    .style(|_| text_style(Color::WHITE)),
                visibility_control,
                details,
                actions
            ]
            .spacing(18),
        )
        .padding(20)
        .width(Length::Fill)
        .style(|_| card_style()),
    )
    .on_enter(Message::HoverOperation(Some(summary.index)))
    .on_exit(Message::HoverOperation(None))
    .into()
}

fn info_row(label: &'static str, value: String) -> Element<'static, Message> {
    row![
        text(label)
            .size(14)
            .style(|_| text_style(MUTED_TEXT))
            .width(Length::Fixed(90.0)),
        text(value).size(16).style(|_| text_style(Color::WHITE)),
    ]
    .spacing(8)
    .into()
}

fn status_badge(status: &ToolpathStatus) -> Element<'static, Message> {
    let (label, color) = match status {
        ToolpathStatus::Dirty => ("Needs regen", Color::from_rgb8(0xF0, 0xA5, 0x2F)),
        ToolpathStatus::Ready { warning_count, .. } => {
            if *warning_count > 0 {
                ("Ready · warnings", Color::from_rgb8(0xF0, 0xC7, 0x3C))
            } else {
                ("Ready", Color::from_rgb8(0x2A, 0xA8, 0x5E))
            }
        }
        ToolpathStatus::Invalid { .. } => ("Invalid", Color::from_rgb8(0xE5, 0x54, 0x54)),
    };

    container(text(label).size(14).style(|_| text_style(Color::WHITE)))
        .padding([4, 12])
        .style(move |_| badge_style(color))
        .into()
}

fn color_swatch(color: Color) -> Element<'static, Message> {
    container(text(""))
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .style(move |_| iced::widget::container::Style {
            background: Some(color.into()),
            border: Border {
                radius: 4.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..Default::default()
        })
        .into()
}

fn badge_style(color: Color) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(color.into()),
        border: Border {
            radius: 999.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

fn panel_style() -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(PANEL_BG.into()),
        border: Border {
            radius: 20.0.into(),
            width: 1.0,
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.05),
        },
        ..Default::default()
    }
}

fn card_style() -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(CARD_BG.into()),
        border: Border {
            radius: 18.0.into(),
            width: 1.0,
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.04),
        },
        ..Default::default()
    }
}

fn text_style(color: Color) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(color),
        ..Default::default()
    }
}
