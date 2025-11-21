use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Color, Element, Length};
use rcarve::StockSpec;

use super::Message;
use super::util::{format_dimension, parse_dimension, parse_origin};

#[derive(Debug, Clone)]
pub struct StockForm {
    pub width: String,
    pub height: String,
    pub thickness: String,
    pub material: String,
    pub origin: String,
    pub error: Option<String>,
}

impl Default for StockForm {
    fn default() -> Self {
        Self {
            width: String::new(),
            height: String::new(),
            thickness: String::new(),
            material: String::new(),
            origin: String::new(),
            error: None,
        }
    }
}

impl StockForm {
    pub fn from_stock(stock: &StockSpec) -> Self {
        Self {
            width: format_dimension(stock.width),
            height: format_dimension(stock.height),
            thickness: format_dimension(stock.thickness),
            material: stock.material.clone().unwrap_or_default(),
            origin: stock
                .origin
                .map(|(x, y, z)| {
                    format!(
                        "{},{},{}",
                        format_dimension(x),
                        format_dimension(y),
                        format_dimension(z)
                    )
                })
                .unwrap_or_default(),
            error: None,
        }
    }

    pub fn parse(&self) -> Result<StockSpec, String> {
        let width = parse_dimension(&self.width, "Width")?;
        let height = parse_dimension(&self.height, "Height")?;
        let thickness = parse_dimension(&self.thickness, "Thickness")?;

        let material = {
            let trimmed = self.material.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        };

        let origin = parse_origin(&self.origin)?;

        Ok(StockSpec {
            width,
            height,
            thickness,
            material,
            origin,
        })
    }
}

pub fn modal(form: &StockForm) -> Element<'_, Message> {
    let width = column![
        text("Width (mm)").size(12),
        text_input("Width", &form.width)
            .on_input(Message::StockWidthChanged)
            .padding(8),
    ]
    .spacing(4);

    let height = column![
        text("Height (mm)").size(12),
        text_input("Height", &form.height)
            .on_input(Message::StockHeightChanged)
            .padding(8),
    ]
    .spacing(4);

    let thickness = column![
        text("Thickness (mm)").size(12),
        text_input("Thickness", &form.thickness)
            .on_input(Message::StockThicknessChanged)
            .padding(8),
    ]
    .spacing(4);

    let material = column![
        text("Material").size(12),
        text_input("Optional", &form.material)
            .on_input(Message::StockMaterialChanged)
            .padding(8),
    ]
    .spacing(4);

    let origin = column![
        text("Origin").size(12),
        text_input("Optional", &form.origin)
            .on_input(Message::StockOriginChanged)
            .padding(8),
    ]
    .spacing(4);

    let mut content = column![
        text("Edit Stock").size(24),
        column![width, height, thickness, material, origin].spacing(12),
    ]
    .spacing(16);

    if let Some(error) = &form.error {
        let color = Color::from_rgb8(0xE5, 0x54, 0x54);
        content = content.push(text(error).style(move |_theme| iced::widget::text::Style {
            color: Some(color),
            ..Default::default()
        }));
    }

    content = content.push(
        row![
            button("Cancel").on_press(Message::CloseStockModal),
            button("Save").on_press(Message::SaveStock),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    );

    container(content)
        .padding(24)
        .width(Length::Fixed(360.0))
        .style(container::rounded_box)
        .into()
}
