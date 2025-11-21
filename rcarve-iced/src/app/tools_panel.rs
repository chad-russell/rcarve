use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};
use rcarve::{Tool, ToolLibrary, ToolType};

use super::Message;

pub fn tools_view(library: &ToolLibrary) -> Element<'static, Message> {
    let content = if library.tools.is_empty() {
        column![
            tools_header(),
            text("No tools saved yet.").size(14),
            text("Use the button above to add your first tool.").size(12),
        ]
        .spacing(8)
    } else {
        let cards = library
            .tools
            .iter()
            .enumerate()
            .fold(column![].spacing(8), |col, (index, tool)| {
                col.push(tool_card(index, tool))
            });

        column![tools_header(), cards].spacing(12)
    };

    container(content)
        .padding(16)
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}

fn tools_header() -> iced::widget::Row<'static, Message> {
    row![
        text("Tool Library").size(20),
        button("+ Add Tool")
            .on_press(Message::AddTool)
            .padding([6, 12]),
    ]
    .align_y(Alignment::Center)
    .spacing(12)
}

fn tool_card(index: usize, tool: &Tool) -> Element<'static, Message> {
    let info = column![
        text(tool.name.clone()).size(16),
        text(tool_description(tool)).size(12),
        text(format!(
            "Stepover: {:.2} • Pass depth: {:.2} mm",
            tool.stepover, tool.pass_depth
        ))
        .size(12),
    ]
    .spacing(4)
    .width(Length::Fill);

    let actions = row![
        button("Edit")
            .padding([4, 10])
            .on_press(Message::EditTool(index)),
        button("Delete")
            .padding([4, 10])
            .on_press(Message::DeleteTool(index)),
    ]
    .spacing(8);

    let content = row![info, actions]
        .align_y(Alignment::Center)
        .spacing(12)
        .width(Length::Fill);

    container(content)
        .padding(12)
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}

fn tool_description(tool: &Tool) -> String {
    match &tool.tool_type {
        ToolType::Endmill { diameter } => format!("Endmill • {:.2} mm", diameter),
        ToolType::Ballnose { diameter } => format!("Ballnose • {:.2} mm", diameter),
        ToolType::VBit { angle_degrees } => {
            format!(
                "V-bit • {}° • Diameter {:.2} mm",
                angle_degrees, tool.diameter
            )
        }
    }
}
