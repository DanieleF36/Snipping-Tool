use iced::widget::canvas::{Cursor, Program};
use iced::{Color, Point, Rectangle, Renderer, Size};
use iced_graphics::Primitive;
use image::{imageops, RgbaImage};

// Internamente iced inverte i colori rosso e blu quando si usa il backend
// software perché una delle dipendenze (winit) usa colori di formato (B, G, R, A).
// https://github.com/iced-rs/iced/blob/b5f102c55835cf42427f9f8672634e81a5d724f6/tiny_skia/src/geometry.rs#L230
fn adjust_color(color: Color) -> Color {
    Color::from_rgba(color.b, color.g, color.r, color.a)
}

fn adjust_primitive_colors(primitive: Primitive) -> Primitive {
    match primitive {
        Primitive::Group { primitives } => Primitive::Group {
            primitives: primitives
                .iter()
                .map(|p| adjust_primitive_colors(p.clone()))
                .collect(),
        },
        Primitive::Cache { content } => Primitive::Cache {
            content: std::sync::Arc::new(adjust_primitive_colors((*content).clone())),
        },
        Primitive::Clip { bounds, content } => Primitive::Clip {
            bounds,
            content: Box::new(adjust_primitive_colors(*content)),
        },
        Primitive::Translate {
            translation,
            content,
        } => Primitive::Translate {
            translation,
            content: Box::new(adjust_primitive_colors(*content)),
        },
        Primitive::Text {
            content,
            bounds,
            color,
            size,
            line_height,
            font,
            horizontal_alignment,
            vertical_alignment,
            shaping,
        } => {
            Primitive::Text {
                content,
                bounds,
                size,
                line_height,
                font,
                horizontal_alignment,
                vertical_alignment,
                shaping,
                color: adjust_color(color),
            }
        }
        Primitive::Stroke {
            ref path,
            ref paint,
            ref stroke,
            transform,
        } => {
            let tiny_skia::Shader::SolidColor(color) = paint.shader else { return primitive };
            let new_color = tiny_skia::Color::from_rgba(
                color.blue(),
                color.green(),
                color.red(),
                color.alpha(),
            )
            .unwrap();
            Primitive::Stroke {
                path: path.clone(),
                stroke: stroke.clone(),
                transform,
                paint: tiny_skia::Paint {
                    shader: tiny_skia::Shader::SolidColor(new_color),
                    ..*paint
                },
            }
        }
        Primitive::Fill { ref path, ref paint, rule, transform } => {
            let tiny_skia::Shader::SolidColor(color) = paint.shader else { return primitive };
            let new_color = tiny_skia::Color::from_rgba(
                color.blue(),
                color.green(),
                color.red(),
                color.alpha(),
            )
            .unwrap();
            Primitive::Fill {
                path: path.clone(),
                rule,
                transform,
                paint: tiny_skia::Paint {
                    shader: tiny_skia::Shader::SolidColor(new_color),
                    ..*paint
                },
            }
        }
        Primitive::Quad {
            bounds,
            background,
            border_radius,
            border_width,
            border_color,
        } => {
            let iced::Background::Color(bc) = background else { return primitive };
            Primitive::Quad {
                bounds,
                background: iced::Background::Color(adjust_color(bc)),
                border_radius,
                border_width,
                border_color: adjust_color(border_color),
            }
        }
        Primitive::Svg {
            ref handle,
            color,
            bounds,
        } => {
            let Some(c) = color else { return primitive };
            Primitive::Svg {
                handle: handle.clone(),
                color: Some(adjust_color(c)),
                bounds,
            }
        }
        Primitive::SolidMesh { buffers, size } => Primitive::SolidMesh {
            size,
            buffers: iced_graphics::primitive::Mesh2D {
                indices: buffers.indices,
                vertices: buffers
                    .vertices
                    .iter()
                    .map(|v| iced_graphics::primitive::ColoredVertex2D {
                        position: v.position,
                        color: [v.color[2], v.color[1], v.color[0], v.color[3]],
                    })
                    .collect(),
            },
        },
        _ => {
            primitive
        }
    }
}

pub fn draw_on_buffer<P, Message>(
    program: &P,
    image: &RgbaImage,
    crop_rec: Option<Rectangle<u32>>,
) -> Option<RgbaImage>
where
    P: Program<Message, Renderer>,
{
    let bounds = Rectangle::new(
        Point::new(0.0, 0.0),
        Size::new(image.width() as f32, image.height() as f32),
    );

    // Internally, this backend is never actually used.
    let backend = iced_tiny_skia::Backend::new(iced_tiny_skia::Settings::default());
    let renderer = Renderer::new(iced_renderer::Backend::TinySkia(backend));

    let cursor = Cursor::Unavailable;

    let geo = program.draw(
        &P::State::default(),
        &renderer,
        &iced::Theme::default(),
        bounds,
        cursor,
    );
    let primitives: Vec<iced_graphics::Primitive> = {
        let mut tmp: Vec<iced_graphics::Primitive> = Vec::new();
        for g in geo {
            tmp.push(adjust_primitive_colors(g.into()));
        }
        tmp
    };

    let mut fg_pixmap = tiny_skia::Pixmap::new(image.width(), image.height())?;
    let mut mask = tiny_skia::Mask::new(image.width(), image.height())?;
    let viewport =
        iced_graphics::Viewport::with_physical_size(Size::new(image.width(), image.height()), 1.0);
    let damage = [bounds];
    let overlay: [String; 0] = [];

    // Create a new backend because the previous one was moved
    let mut backend = iced_tiny_skia::Backend::new(Default::default());

    #[cfg(test)]
    {
        use iced_graphics::backend::Text;
        backend.load_font(
            std::borrow::Cow::Borrowed(include_bytes!("../tests/tektur.ttf"))
        );
    }

    backend.draw(
        &mut fg_pixmap.as_mut(),
        &mut mask,
        primitives.as_slice(),
        &viewport,
        &damage,
        iced::Color::TRANSPARENT,
        &overlay,
    );

    let size = tiny_skia_path::IntSize::from_wh(image.width(), image.height())?;
    let mut bg_pixmap = tiny_skia::Pixmap::from_vec(image.to_vec(), size)?;

    bg_pixmap.as_mut().draw_pixmap(
        0,
        0,
        fg_pixmap.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::default(),
        None,
    );

    let mut ret = RgbaImage::from_raw(image.width(), image.height(), bg_pixmap.data().to_vec())?;

    if let Some(cr) = crop_rec {
        return Some(imageops::crop(&mut ret, cr.x, cr.y, cr.width, cr.height).to_image());
    } else {
        return Some(ret);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::canvas::{Cursor, Frame, Geometry, Path, Program};
    use iced::Theme;
    use image::io::Reader;

    // Messaggio dummy per i test
    enum Message {}

    #[derive(Debug)]
    pub struct SimpleAnnotations {
        radius: f32,
    }

    impl SimpleAnnotations {
        pub fn new(r: f32) -> Self {
            Self { radius: r }
        }
    }

    impl<Message> Program<Message, Renderer> for SimpleAnnotations {
        type State = ();

        fn draw(
            &self,
            _state: &Self::State,
            renderer: &Renderer,
            _theme: &Theme,
            bounds: Rectangle,
            _cursor: Cursor,
        ) -> Vec<Geometry> {
            let mut frame = Frame::new(renderer, bounds.size());
            let height = frame.height();

            let circle = Path::circle(Point::new(0.0, 0.0), self.radius * height / 100.0);
            let circle1 = Path::circle(
                Point::new(frame.width(), frame.height()),
                self.radius * height / 100.0,
            );
            let mut text = iced::widget::canvas::Text::from("Test");
            text.position = frame.center();
            text.color = Color::from_rgba(1.0, 0.0, 0.0, 1.0);
            text.size = 15.0 * height / 100.0;
            text.font = iced::Font::with_name("Tektur");

            let circle_color = Color::from_rgba(0.0, 0.0, 1.0, 1.0);
            frame.fill(&circle, circle_color);
            frame.fill(&circle1, circle_color);
            frame.fill_text(text);

            vec![frame.into_geometry()]
        }
    }

    #[test]
    fn canvas_to_pixel_no_crop() {
        let dyn_image = Reader::open("tests/screenshot.png")
            .unwrap()
            .decode()
            .unwrap();
        let control_dyn_image = Reader::open("tests/annotated_screenshot1.png")
            .unwrap()
            .decode()
            .unwrap();

        let image = dyn_image.to_rgba8();

        let annotations = SimpleAnnotations::new(25.0);

        let result =
            draw_on_buffer::<SimpleAnnotations, Message>(&annotations, &image, None).unwrap();
        // _ = result.save("tests/annotated_screenshot_new.png");

        let control_image = control_dyn_image.to_rgba8();

        assert!(
            result == control_image,
            "The generated image is different from the control one"
        );
    }

    #[test]
    fn canvas_to_pixel_with_crop() {
        let dyn_image = Reader::open("tests/screenshot.png")
            .unwrap()
            .decode()
            .unwrap();
        let control_dyn_image = Reader::open("tests/annotated_screenshot_cropped.png")
            .unwrap()
            .decode()
            .unwrap();

        let image = dyn_image.to_rgba8();

        let annotations = SimpleAnnotations::new(25.0);

        let crop_rec = Rectangle {
            x: 1500,
            y: 818,
            width: 1200,
            height: 800,
        };
        let result =
            draw_on_buffer::<SimpleAnnotations, Message>(&annotations, &image, Some(crop_rec))
                .unwrap();

        let control_image = control_dyn_image.to_rgba8();
        // _ = result.save("tests/annotated_screenshot_cropped_new.png");

        assert!(
            result == control_image,
            "The generated image is different from the control one"
        );
    }

    #[test]
    fn adjust_color_test() {
        let test_color = Color::new(1.0, 0.1, 0.2, 0.3);
        let control_color = Color::new(0.2, 0.1, 1.0, 0.3);

        let res = adjust_color(test_color);

        assert_eq!(control_color, res);
    }
}
