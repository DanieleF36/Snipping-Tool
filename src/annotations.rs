use iced::mouse;
use iced::widget::canvas::{
    event, path, Cursor, Event, Frame, Geometry, Path, Program, Style, Text,
};
use iced::widget::canvas::{LineCap, LineDash, LineJoin, Stroke};
use iced::{Theme, Font};
use iced::{Color, Point, Rectangle, Renderer, Size, Vector};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum FillStyle {
    Fill,
    Stroke(f32),
}

#[derive(Debug, Clone)]
pub enum Tool {
    Rectangle {
        color: Color,
        fill_style: FillStyle,
    },
    Arrow {
        color: Color,
        stroke_width: f32,
    },
    Text {
        color: Color,
        content: String,
        size: f32,
        font: Font
    },
    FreeHand {
        color: Color,
        stroke_width: f32,
    },
}

#[derive(Clone)]
enum PrivTool {
    Rectangle {
        color: Color,
        fill_style: FillStyle,
        rec: Rectangle,
    },
    Arrow {
        color: Color,
        stroke_width: f32,
        begin: Point,
        end: Point,
    },
    Text(Text),
    FreeHand {
        color: Color,
        stroke_width: f32,
        points: Vec<Point>,
    },
}

impl From<Tool> for PrivTool {
    fn from(tool: Tool) -> Self {
        match tool {
            Tool::Rectangle { color, fill_style } => PrivTool::Rectangle {
                color,
                fill_style,
                rec: Rectangle::default(),
            },
            Tool::Arrow {
                color,
                stroke_width,
            } => PrivTool::Arrow {
                color,
                stroke_width,
                begin: Point::default(),
                end: Point::default(),
            },
            Tool::Text {
                color,
                content,
                size,
                font
            } => PrivTool::Text(Text {
                content,
                color,
                size,
                horizontal_alignment: iced::alignment::Horizontal::Center,
                vertical_alignment: iced::alignment::Vertical::Center,
                font,
                ..Text::default()
            }),
            Tool::FreeHand {
                color,
                stroke_width,
            } => PrivTool::FreeHand {
                color,
                stroke_width,
                points: Vec::with_capacity(300),
            },
        }
    }
}

#[derive(Clone)]
pub struct Annotations<Message: Clone> {
    tool: Rc<RefCell<Option<PrivTool>>>,
    baked_geometry: Rc<RefCell<Vec<PrivTool>>>,
    image_size: Size<f32>,
    crop_area: Rectangle<f32>,
    new_annotation_msg: Message,
}

#[derive(Debug, Default)]
pub struct State {
    mouse_pressed: bool,
    cache: iced::widget::canvas::Cache,
}

impl<Message: Clone> Annotations<Message> {
    pub fn new(image_size: Size<f32>, new_annotation_msg: Message) -> Self {
        Self {
            tool: Rc::new(None.into()),
            baked_geometry: Default::default(),
            image_size,
            crop_area: Rectangle::with_size(image_size),
            new_annotation_msg,
        }
    }

    pub fn undo_annotation(&mut self) {
        self.baked_geometry.borrow_mut().pop();
    }

    pub fn clear_annotations(&mut self) {
        self.baked_geometry.borrow_mut().clear();
        *self.tool.borrow_mut() = None;
        self.crop_area = Rectangle::with_size(self.image_size);
    }

    pub fn set_tool(&mut self, tool: Option<Tool>) {
        if let Some(t) = tool {
            *self.tool.borrow_mut() = Some(t.into());
        } else {
            *self.tool.borrow_mut() = None;
        }
    }

    pub fn set_crop(&mut self, rec: Rectangle<f32>) -> Rectangle<f32> {
        let ret = self.crop_area;
        self.crop_area = rec;
        return ret;
    }

    pub fn set_image_size(&mut self, size: Size<f32>) {
        self.image_size = size;
        self.crop_area = Rectangle::with_size(size);
    }

    fn paint(
        &self,
        frame: &mut Frame,
        tool: &PrivTool,
        scale: f32,
        translation_vector: &Vector<f32>,
    ) {
        use iced_graphics::geometry::path::lyon_path;
        let v = lyon_path::math::vector(translation_vector.x, translation_vector.y);
        let t = lyon_path::math::Transform::identity()
            .pre_scale(scale, scale)
            .pre_translate(v);

        match tool {
            PrivTool::Text(txt) => {
                let tmp_txt = Text {
                    color: txt.color,
                    size: txt.size * scale * 25.0,
                    position: iced::Point::new(txt.position.x, txt.position.y),
                    ..txt.clone()
                };

                frame.with_clip(Rectangle::with_size(frame.size()), |f| {
                    f.scale(scale);
                    f.translate(*translation_vector);
                    f.fill_text(tmp_txt)
                });
            }
            PrivTool::Rectangle {
                color,
                rec,
                fill_style,
            } => {
                let top_left = Point::new(rec.x, rec.y);
                let size = Size::new(rec.width, rec.height);
                let rec_path = Path::rectangle(top_left, size).transform(&t);

                match fill_style {
                    FillStyle::Fill => frame.fill(&rec_path, *color),
                    FillStyle::Stroke(width) => {
                        let stroke = Stroke {
                            style: Style::Solid(*color),
                            width: *width * scale * 25.0,
                            line_cap: LineCap::Round,
                            line_join: LineJoin::Round,
                            line_dash: LineDash::default(),
                        };

                        frame.with_clip(Rectangle::with_size(frame.size()), |f| {
                            f.stroke(&rec_path, stroke);
                        });
                    }
                }
            }
            PrivTool::Arrow {
                color,
                stroke_width,
                begin,
                end,
            } => {
                let mut builder = path::Builder::new();
                let start_point = Point::new(begin.x, begin.y);
                let end_point = Point::new(end.x, end.y);

                let arrow_angle = (end_point.y - start_point.y).atan2(end_point.x - start_point.x);

                let upper_angle =
                    arrow_angle + std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_3;
                let upper_stroke_point = Point::new(
                    end_point.x + upper_angle.cos() * 0.05 * self.image_size.height,
                    end_point.y + upper_angle.sin() * 0.05 * self.image_size.height,
                );

                let lower_angle =
                    arrow_angle - std::f32::consts::FRAC_PI_2 - std::f32::consts::FRAC_PI_3;
                let lower_stroke_point = Point::new(
                    end_point.x + lower_angle.cos() * 0.05 * self.image_size.height,
                    end_point.y + lower_angle.sin() * 0.05 * self.image_size.height,
                );

                let stroke = Stroke {
                    style: Style::Solid(*color),
                    width: *stroke_width * scale * 25.0,
                    line_cap: LineCap::Round,
                    line_join: LineJoin::Round,
                    line_dash: LineDash::default(),
                };

                builder.move_to(start_point);
                builder.line_to(end_point);
                if start_point.distance(end_point) > 0.0 {
                    builder.line_to(upper_stroke_point);
                    builder.move_to(end_point);
                    builder.line_to(lower_stroke_point);
                }

                let arrow = builder.build().transform(&t);

                //                frame.stroke(&arrow, stroke);
                frame.with_clip(Rectangle::with_size(frame.size()), |f| {
                    f.stroke(&arrow, stroke);
                });
            }
            PrivTool::FreeHand {
                points,
                color,
                stroke_width,
            } => {
                let mut builder = path::Builder::new();

                if let Some(first) = points.first() {
                    builder.move_to(*first);

                    for p in &points[1..] {
                        builder.line_to(*p);
                    }

                    let path = builder.build().transform(&t);

                    let stroke = Stroke {
                        style: Style::Solid(*color),
                        width: *stroke_width * scale * 25.0,
                        line_cap: LineCap::Round,
                        line_join: LineJoin::Round,
                        line_dash: LineDash::default(),
                    };

                    //                    frame.stroke(&path, stroke);
                    frame.with_clip(Rectangle::with_size(frame.size()), |f| {
                        f.stroke(&path, stroke);
                    });
                }
            }
        };
    }
}

//Specifica come le cose vanno disegnate dentro il canvas
impl<Message: Clone> Program<Message, Renderer> for Annotations<Message> {
    type State = State;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (event::Status, Option<Message>) {
        state.cache.clear();

        let cursor_position = if let Some(cp) = cursor.position_in(&bounds) {
            Point::new(
                cp.x / bounds.width * self.crop_area.width + self.crop_area.x,
                cp.y / bounds.height * self.crop_area.height + self.crop_area.y,
            )
        } else {
            return (event::Status::Ignored, None);
        };

        let mut opt_tool = self.tool.borrow_mut();
        let tool: &mut PrivTool = if let Some(t) = opt_tool.as_mut() {
            t
        } else {
            return (event::Status::Ignored, None);
        };

        match tool {
            PrivTool::Text(txt) => {
                let text = Text {
                    position: cursor_position,
                    ..txt.clone()
                };
                *tool = PrivTool::Text(text);
            }
            _ => (),
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.mouse_pressed = true;
                match tool {
                    PrivTool::Rectangle { ref mut rec, .. } => {
                        rec.x = cursor_position.x;
                        rec.y = cursor_position.y;
                    }
                    PrivTool::Arrow {
                        ref mut begin,
                        ref mut end,
                        ..
                    } => {
                        *begin = cursor_position;
                        *end = cursor_position;
                    }
                    PrivTool::Text(_) => {}
                    PrivTool::FreeHand { ref mut points, .. } => points.push(cursor_position),
                }
                return (event::Status::Captured, None);
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.mouse_pressed = false;

                self.baked_geometry.borrow_mut().push(tool.clone().into());

                // Reset tools to avoid showing glitchy annotations
                match tool {
                    PrivTool::Rectangle { ref mut rec, .. } => *rec = Rectangle::default(),
                    PrivTool::Arrow {
                        ref mut begin,
                        ref mut end,
                        ..
                    } => {
                        *begin = Point::ORIGIN;
                        *end = Point::ORIGIN;
                    }
                    PrivTool::Text(ref mut txt) => {
                        txt.position = Point::ORIGIN;
                    }
                    PrivTool::FreeHand { ref mut points, .. } => points.clear(),
                }
                return (
                    event::Status::Captured,
                    Some(self.new_annotation_msg.clone()),
                );
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.mouse_pressed {
                    match tool {
                        PrivTool::Text(_) => {}
                        PrivTool::Rectangle { ref mut rec, .. } => {
                            rec.width = cursor_position.x - rec.x;
                            rec.height = cursor_position.y - rec.y;
                        }
                        PrivTool::Arrow { ref mut end, .. } => {
                            *end = cursor_position;
                        }
                        PrivTool::FreeHand { ref mut points, .. } => {
                            if let Some(p) = points.last() {
                                if p.distance(cursor_position) > 10.0 {
                                    points.push(cursor_position);
                                }
                            }
                        }
                    }
                }
                //                state.cache.clear();
                return (event::Status::Captured, None);
            }
            _ => return (event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Vec<Geometry> {
        //let scale = self.crop_area.height / self.image_size.height / bounds.height * self.image_size.height;
        let scale =
            self.image_size.height * bounds.height / self.crop_area.height / self.image_size.height;
        let translation_vector = Vector::new(-self.crop_area.x, -self.crop_area.y);

        let g = state.cache.draw(renderer, bounds.size(), |f| {
            for t in self.baked_geometry.borrow().iter() {
                self.paint(f, t, scale, &translation_vector);
            }

            if cursor.is_over(&bounds) {
                if let Some(t) = self.tool.borrow().as_ref() {
                    self.paint(f, t, scale, &translation_vector);
                }
            }
        });

        vec![g]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(&bounds) && self.tool.borrow().is_some() {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::io::Reader;
    use crate::canvas_to_pixels::draw_on_buffer;

    // Messaggio dummy per i test
    #[derive(Clone)]
    enum Message {
        Dummy
    }

    #[test]
    fn annotations_text() {
        let dyn_image = Reader::open("tests/screenshot.png")
            .unwrap()
            .decode()
            .unwrap();
        let control_dyn_image = Reader::open("tests/annotated_screenshot_text.png")
            .unwrap()
            .decode()
            .unwrap();

        let image = dyn_image.to_rgba8();

        let image_size = Size::new(image.width() as f32, image.height() as f32);
        let mut state = State::default();
        let mut annotations = Annotations::new(
            image_size,
            Message::Dummy
        );

        let tool = Tool::Text {
            color: Color::new(1.0, 1.0, 0.0, 1.0),
            content: "Test".to_string(),
            size: 15.0,
            font: iced::Font::with_name("Tektur")
        };
        annotations.set_tool(Some(tool));

        let cursor_pos = Point::new(500.0, 500.0);
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: cursor_pos }),
            Rectangle::with_size(image_size),
            Cursor::Available(cursor_pos)
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            Rectangle::with_size(image_size),
            Cursor::Available(cursor_pos)
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            Rectangle::with_size(image_size),
            Cursor::Available(cursor_pos)
        );

        let result =
            draw_on_buffer::<Annotations<Message>, Message>(&annotations, &image, None).unwrap();
        // _ = result.save("tests/annotated_screenshot_text_new.png");

        let control_image = control_dyn_image.to_rgba8();

        assert!(
            result == control_image,
            "The generated image is different from the control one"
        );
    }

    #[test]
    fn annotations_free_hand() {
        let dyn_image = Reader::open("tests/screenshot.png")
            .unwrap()
            .decode()
            .unwrap();
        let control_dyn_image = Reader::open("tests/annotated_screenshot_free_hand.png")
            .unwrap()
            .decode()
            .unwrap();

        let image = dyn_image.to_rgba8();

        let image_size = Size::new(image.width() as f32, image.height() as f32);
        let mut state = State::default();
        let mut annotations = Annotations::new(
            image_size,
            Message::Dummy
        );

        let tool = Tool::FreeHand {
            color: Color::new(1.0, 0.5, 0.5, 1.0),
            stroke_width: 1.0
        };
        annotations.set_tool(Some(tool));

        let cursor_pos = Point::new(500.0, 500.0);
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: cursor_pos }),
            Rectangle::with_size(image_size),
            Cursor::Available(cursor_pos)
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            Rectangle::with_size(image_size),
            Cursor::Available(cursor_pos)
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(600.0, 600.0) }),
            Rectangle::with_size(image_size),
            Cursor::Available(Point::new(600.0, 600.0))
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(600.0, 500.0) }),
            Rectangle::with_size(image_size),
            Cursor::Available(Point::new(600.0, 500.0))
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            Rectangle::with_size(image_size),
            Cursor::Available(Point::new(400.0, 400.0))
        );

        let result =
            draw_on_buffer::<Annotations<Message>, Message>(&annotations, &image, None).unwrap();
        // _ = result.save("tests/annotated_screenshot_free_hand_new.png");

        let control_image = control_dyn_image.to_rgba8();

        assert!(
            result == control_image,
            "The generated image is different from the control one"
        );
    }

    #[test]
    fn annotations_arrow() {
        let dyn_image = Reader::open("tests/screenshot.png")
            .unwrap()
            .decode()
            .unwrap();
        let control_dyn_image = Reader::open("tests/annotated_screenshot_arrow.png")
            .unwrap()
            .decode()
            .unwrap();

        let image = dyn_image.to_rgba8();

        let image_size = Size::new(image.width() as f32, image.height() as f32);
        let mut state = State::default();
        let mut annotations = Annotations::new(
            image_size,
            Message::Dummy
        );

        let tool = Tool::Arrow {
            color: Color::new(1.0, 0.5, 0.5, 1.0),
            stroke_width: 1.0
        };
        annotations.set_tool(Some(tool));

        let cursor_pos = Point::new(500.0, 500.0);
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: cursor_pos }),
            Rectangle::with_size(image_size),
            Cursor::Available(cursor_pos)
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            Rectangle::with_size(image_size),
            Cursor::Available(cursor_pos)
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(600.0, 600.0) }),
            Rectangle::with_size(image_size),
            Cursor::Available(Point::new(600.0, 600.0))
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(600.0, 500.0) }),
            Rectangle::with_size(image_size),
            Cursor::Available(Point::new(1000.0, 700.0))
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            Rectangle::with_size(image_size),
            Cursor::Available(Point::new(450.0, 450.0))
        );

        let result =
            draw_on_buffer::<Annotations<Message>, Message>(&annotations, &image, None).unwrap();
        // _ = result.save("tests/annotated_screenshot_arrow_new.png");

        let control_image = control_dyn_image.to_rgba8();

        assert!(
            result == control_image,
            "The generated image is different from the control one"
        );
    }

    #[test]
    fn annotations_rectangle() {
        let dyn_image = Reader::open("tests/screenshot.png")
            .unwrap()
            .decode()
            .unwrap();
        let control_dyn_image = Reader::open("tests/annotated_screenshot_rectangle.png")
            .unwrap()
            .decode()
            .unwrap();

        let image = dyn_image.to_rgba8();

        let image_size = Size::new(image.width() as f32, image.height() as f32);
        let mut state = State::default();
        let mut annotations = Annotations::new(
            image_size,
            Message::Dummy
        );

        let tool = Tool::Rectangle {
            color: Color::new(0.0, 0.5, 0.1, 1.0),
            fill_style: FillStyle::Stroke(1.0)
        };
        annotations.set_tool(Some(tool));

        let cursor_pos = Point::new(500.0, 500.0);
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: cursor_pos }),
            Rectangle::with_size(image_size),
            Cursor::Available(cursor_pos)
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            Rectangle::with_size(image_size),
            Cursor::Available(cursor_pos)
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(600.0, 600.0) }),
            Rectangle::with_size(image_size),
            Cursor::Available(Point::new(600.0, 600.0))
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(600.0, 500.0) }),
            Rectangle::with_size(image_size),
            Cursor::Available(Point::new(1000.0, 700.0))
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            Rectangle::with_size(image_size),
            Cursor::Available(Point::new(450.0, 450.0))
        );

        let result =
            draw_on_buffer::<Annotations<Message>, Message>(&annotations, &image, None).unwrap();
        // _ = result.save("tests/annotated_screenshot_rectangle_new.png");

        let control_image = control_dyn_image.to_rgba8();

        assert!(
            result == control_image,
            "The generated image is different from the control one"
        );
    }

    #[test]
    fn annotations_combined_cropped() {
        let dyn_image = Reader::open("tests/screenshot.png")
            .unwrap()
            .decode()
            .unwrap();
        let control_dyn_image = Reader::open("tests/annotated_screenshot_combined_cropped.png")
            .unwrap()
            .decode()
            .unwrap();

        let image = dyn_image.to_rgba8();

        let image_size = Size::new(image.width() as f32, image.height() as f32);
        let mut state = State::default();
        let mut annotations = Annotations::new(
            image_size,
            Message::Dummy
        );

        let crop_rec = Rectangle::new(
            Point::new(500.0, 0.0),
            Size::new(1100.0, 800.0)
        );
        let bounds = Rectangle::with_size(Size::new(image_size.width / 10.0, image_size.height / 10.0));
        annotations.set_crop(crop_rec);

        let rec_tool = Tool::Rectangle {
            color: Color::new(0.0, 0.5, 0.1, 1.0),
            fill_style: FillStyle::Stroke(1.0)
        };
        let text_tool = Tool::Text {
            color: Color::new(1.0, 1.0, 0.0, 1.0),
            content: "Test".to_string(),
            size: 15.0,
            font: iced::Font::with_name("Tektur")
        };

        // Draw rectangle
        annotations.set_tool(Some(rec_tool));
        let cursor_pos = Point::new(10.0, 20.0);
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: cursor_pos }),
            bounds,
            Cursor::Available(cursor_pos)
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(cursor_pos)
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(50.0, 25.0) }),
            bounds,
            Cursor::Available(Point::new(50.0, 25.0))
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(175.0, 150.0) }),
            bounds,
            Cursor::Available(Point::new(175.0, 150.0))
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(Point::new(175.0, 150.0))
        );

        // Draw text
        annotations.set_tool(Some(text_tool));
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(10.0, 80.0) }),
            bounds,
            Cursor::Available(Point::new(10.0, 80.0))
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(Point::new(10.0, 80.0))
        );
        annotations.update(
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(Point::new(10.0, 80.0))
        );

        annotations.set_crop(Rectangle::with_size(image_size));
        let result =
            draw_on_buffer::<Annotations<Message>, Message>(&annotations, &image, Some(crop_rec.snap())).unwrap();
        // _ = result.save("tests/annotated_screenshot_combined_cropped_new.png");

        let control_image = control_dyn_image.to_rgba8();

        assert!(
            result == control_image,
            "The generated image is different from the control one"
        );
    }
}
