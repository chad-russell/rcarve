use iced::border::Border;
use iced::widget::{button, column, container, mouse_area, row, text};
use iced::{Alignment, Color, Element, Length};
use std::path::Path;
use ulid::Ulid;

use super::Message;
use super::project::ImportedSvgEntry;

pub fn imports_view<'a>(
    imports: &'a [ImportedSvgEntry],
    selected: Option<Ulid>,
    importing_svg: bool,
) -> Element<'a, Message> {
    let heading = row![text("Imported SVGs").size(20), import_button(importing_svg)]
        .spacing(12)
        .align_y(Alignment::Center);

    let body: Element<'a, Message> = if imports.is_empty() {
        column![
            heading,
            text("No SVG files imported yet.").size(14),
            text("Use the button above to add one.").size(12),
        ]
        .spacing(8)
        .into()
    } else {
        let cards = imports
            .iter()
            .map(|import| import_card(import, selected == Some(import.id)));

        column![heading, column(cards).spacing(8)]
            .spacing(12)
            .into()
    };

    container(body)
        .padding(16)
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}

fn import_button(importing: bool) -> Element<'static, Message> {
    let mut button = button(if importing {
        "Importing..."
    } else {
        "Import SVG"
    })
    .padding([6, 12]);

    if !importing {
        button = button.on_press(Message::ImportSvg);
    }

    button.into()
}

fn import_card(import: &ImportedSvgEntry, selected: bool) -> Element<'_, Message> {
    let background = if selected {
        Color::from_rgb8(0x2a, 0x64, 0xc5)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.05)
    };

    let source_label = import
        .source_path
        .as_deref()
        .and_then(|p| Path::new(p).file_name().and_then(|name| name.to_str()))
        .or(import.source_path.as_deref())
        .unwrap_or("Unknown source");

    let counts = format!(
        "{} curves • {} shapes",
        import.curve_ids.len(),
        import.shape_ids.len()
    );

    let content = column![
        row![
            column![
                text(&import.label).size(16),
                text(source_label).size(12),
                text(counts).size(12),
            ]
            .spacing(4)
            .width(Length::Fill),
            button("Delete")
                .padding([6, 12])
                .on_press(Message::DeleteImport(import.id)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .width(Length::Fill);

    let card = container(content)
        .padding(12)
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: 8.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..container::Style::default()
        });

    mouse_area(card)
        .on_press(Message::SelectImport(import.id))
        .into()
}
