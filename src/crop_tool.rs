use iced::widget::canvas::{
    event, path::Builder, Cache, Cursor, Event, Geometry, LineCap, LineDash, LineJoin, Path,
    Program, Stroke, Style,
};
pub use iced::Rectangle;
use iced::{
    mouse::{
        self,
        Event::{ButtonPressed, ButtonReleased, CursorMoved},
    },
    Color, Point, Renderer, Size, Theme,
};

use std::cell::Cell;

const HANDLE_SIZE: f32 = 17.0;

pub struct CropTool {
    crop_rectangle: Cell<Option<Rectangle>>,
    crop_bounds: Rectangle<f32>,
    min_crop_size: Size<f32>,
}

#[derive(Debug)]
enum CropAction {
    ResizeTl,
    ResizeBr,
    MoveArea,
}

#[derive(Debug, Default)]
pub struct State {
    crop_rectangle: Option<Rectangle>,
    tl_handle: Rectangle,
    br_handle: Rectangle,
    action: Option<CropAction>,
    click_offset: Point,
    cache: Cache,
}

impl CropTool {
    pub fn new(crop_bounds: Rectangle<f32>, min_crop_size: Size<f32>) -> Self {
        Self {
            crop_rectangle: None.into(),
            crop_bounds,
            min_crop_size,
        }
    }

    pub fn get_crop_rec(&self) -> Option<Rectangle> {
        return self.crop_rectangle.get();
    }
}

impl<Message: Clone> Program<Message, Renderer> for CropTool {
    type State = State;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (event::Status, Option<Message>) {
        let cursor_position = if let Some(cp) = cursor.position_in(&bounds) {
            Point::new(
                cp.x * self.crop_bounds.width / bounds.width + self.crop_bounds.x,
                cp.y * self.crop_bounds.height / bounds.height + self.crop_bounds.y,
            )
        } else {
            return (event::Status::Ignored, None);
        };

        if let Some(rec) = &mut state.crop_rectangle {
            let handle_size = HANDLE_SIZE * self.crop_bounds.height / bounds.height;

            state.tl_handle = Rectangle::new(
                Point::new(rec.x - handle_size / 2.0, rec.y - handle_size / 2.0),
                Size::new(handle_size, handle_size),
            );

            state.br_handle = Rectangle::new(
                Point::new(
                    rec.x + rec.width - handle_size / 2.0,
                    rec.y + rec.height - handle_size / 2.0,
                ),
                Size::new(handle_size, handle_size),
            );

            match event {
                Event::Mouse(ButtonPressed(mouse::Button::Left)) => {
                    state.action = if state.tl_handle.contains(cursor_position) {
                        Some(CropAction::ResizeTl)
                    } else if state.br_handle.contains(cursor_position) {
                        Some(CropAction::ResizeBr)
                    } else if rec.contains(cursor_position) {
                        state.click_offset = Point {
                            x: cursor_position.x - rec.x,
                            y: cursor_position.y - rec.y,
                        };
                        Some(CropAction::MoveArea)
                    } else {
                        None
                    };
                    return (event::Status::Captured, None);
                }
                Event::Mouse(CursorMoved { .. }) => {
                    if let Some(action) = &state.action {
                        match action {
                            CropAction::ResizeTl => {
                                let prev_br = Point::new(rec.x + rec.width, rec.y + rec.height);

                                rec.x = if rec.width - (cursor_position.x - rec.x)
                                    < self.min_crop_size.width
                                {
                                    rec.x + rec.width - self.min_crop_size.width
                                } else {
                                    cursor_position.x
                                };
                                rec.y = if rec.height - (cursor_position.y - rec.y)
                                    < self.min_crop_size.height
                                {
                                    rec.y + rec.height - self.min_crop_size.height
                                } else {
                                    cursor_position.y
                                };

                                let width = prev_br.x - rec.x;
                                rec.width = if width > self.min_crop_size.width {
                                    width
                                } else {
                                    self.min_crop_size.width
                                };

                                let height = prev_br.y - rec.y;
                                rec.height = if height > self.min_crop_size.height {
                                    height
                                } else {
                                    self.min_crop_size.height
                                };
                            }
                            CropAction::ResizeBr => {
                                let width = cursor_position.x - rec.x;
                                rec.width = if width > self.min_crop_size.width {
                                    width
                                } else {
                                    self.min_crop_size.width
                                };

                                let height = cursor_position.y - rec.y;
                                rec.height = if height > self.min_crop_size.height {
                                    height
                                } else {
                                    self.min_crop_size.height
                                };
                            }
                            CropAction::MoveArea => {
                                let new_x = cursor_position.x - state.click_offset.x;
                                let new_y = cursor_position.y - state.click_offset.y;

                                rec.x = if new_x < self.crop_bounds.x {
                                    self.crop_bounds.x
                                } else if new_x + rec.width
                                    > self.crop_bounds.width + self.crop_bounds.x
                                {
                                    self.crop_bounds.width - rec.width + self.crop_bounds.x
                                } else {
                                    new_x
                                };

                                rec.y = if new_y < self.crop_bounds.y {
                                    self.crop_bounds.y
                                } else if new_y + rec.height
                                    > self.crop_bounds.height + self.crop_bounds.y
                                {
                                    self.crop_bounds.height - rec.height + self.crop_bounds.y
                                } else {
                                    new_y
                                };
                            }
                        }
                        state.cache.clear();
                        return (event::Status::Captured, None);
                    }
                }
                Event::Mouse(ButtonReleased(mouse::Button::Left)) => {
                    state.action = None;
                    state.cache.clear();
                    self.crop_rectangle.set(state.crop_rectangle);
                    return (event::Status::Captured, None);
                }
                _ => (),
            }
        } else if let Event::Mouse(ButtonPressed(mouse::Button::Left)) = event {
            let mut rec = Rectangle {
                x: cursor_position.x,
                y: cursor_position.y,
                width: self.min_crop_size.width,
                height: self.min_crop_size.height,
            };

            if rec.x + rec.width > self.crop_bounds.x + self.crop_bounds.width {
                rec.x = self.crop_bounds.width - rec.width + self.crop_bounds.x;
            }

            if rec.y + rec.height > self.crop_bounds.y + self.crop_bounds.height {
                rec.y = self.crop_bounds.height - rec.height + self.crop_bounds.y;
            }

            state.crop_rectangle = Some(rec);
            state.action = Some(CropAction::ResizeBr);
            return (event::Status::Captured, None);
        }

        return (event::Status::Ignored, None);
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        //let mut frame = Frame::new(renderer, bounds.size());
        let bg_color = Color::new(0.0, 0.0, 0.0, 0.80);

        let geo = state.cache.draw(&renderer, bounds.size(), |frame| {
            if let Some(rec) = state.crop_rectangle {
                let scaled_rec = Rectangle {
                    x: (rec.x - self.crop_bounds.x) * bounds.width / self.crop_bounds.width,
                    y: (rec.y - self.crop_bounds.y) * bounds.height / self.crop_bounds.height,
                    width: rec.width * bounds.width / self.crop_bounds.width,
                    height: rec.height * bounds.height / self.crop_bounds.height,
                };

                let handle_stroke_width = 4.0;

                let rec_br = Point::new(
                    scaled_rec.x + scaled_rec.width,
                    scaled_rec.y + scaled_rec.height,
                );

                let mut builder = Builder::new();
                builder.move_to(Point::new(scaled_rec.x + HANDLE_SIZE, scaled_rec.y));
                builder.line_to(scaled_rec.position());
                builder.line_to(Point::new(scaled_rec.x, scaled_rec.y + HANDLE_SIZE));
                let tl_handle_path = builder.build();

                let mut builder = Builder::new();
                builder.move_to(Point::new(rec_br.x - HANDLE_SIZE, rec_br.y));
                builder.line_to(rec_br);
                builder.line_to(Point::new(rec_br.x, rec_br.y - HANDLE_SIZE));
                let br_handle_path = builder.build();

                frame.fill_rectangle(
                    Point::ORIGIN,
                    Size::new(bounds.width, scaled_rec.y),
                    bg_color,
                );
                frame.fill_rectangle(
                    Point::new(0.0, rec_br.y),
                    Size::new(bounds.width, bounds.height - rec_br.y),
                    bg_color,
                );
                frame.fill_rectangle(
                    Point::new(0.0, scaled_rec.y),
                    Size::new(scaled_rec.x, scaled_rec.height),
                    bg_color,
                );
                frame.fill_rectangle(
                    Point::new(rec_br.x, scaled_rec.y),
                    Size::new(bounds.width - rec_br.x, scaled_rec.height),
                    bg_color,
                );

                let stroke = Stroke {
                    style: Style::Solid(Color::WHITE),
                    width: handle_stroke_width,
                    line_cap: LineCap::Round,
                    line_join: LineJoin::Round,
                    line_dash: LineDash::default(),
                };

                frame.stroke(&tl_handle_path, stroke.clone());
                frame.stroke(&br_handle_path, stroke);
            } else {
                let path = Path::rectangle(Point::ORIGIN, bounds.size());
                frame.fill(&path, bg_color);
            }
        });

        return vec![geo];
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        let cursor_position = if let Some(cp) = cursor.position_in(&bounds) {
            Point::new(
                cp.x * self.crop_bounds.width / bounds.width + self.crop_bounds.x,
                cp.y * self.crop_bounds.height / bounds.height + self.crop_bounds.y,
            )
        } else {
            return mouse::Interaction::default();
        };

        let rec = if let Some(rec) = state.crop_rectangle {
            rec
        } else {
            return mouse::Interaction::Crosshair;
        };

        if state.tl_handle.contains(cursor_position) {
            mouse::Interaction::ResizingVertically
        } else if state.br_handle.contains(cursor_position) {
            mouse::Interaction::ResizingVertically
        } else if rec.contains(cursor_position) && state.action.is_none() {
            mouse::Interaction::Grab
        } else if rec.contains(cursor_position) && state.action.is_some() {
            mouse::Interaction::Grabbing
        } else {
            mouse::Interaction::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Messaggio dummy per i test
    #[derive(Clone)]
    enum Message {
    }

    #[test]
    fn minimum_size() {
        let crop_bounds = Rectangle::new(
            Point::new(0.0, 0.0),
            Size::new(100.0, 200.0),
        );
        let bounds = Rectangle::new(
            Point::new(0.0, 0.0),
            Size::new(100.0, 200.0),
        );

        let min_crop_size = Size::new(10.0, 10.0);

        let mut state = State::default();
        let cr = CropTool::new(crop_bounds, min_crop_size);

        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(10.0, 20.0) }),
            bounds,
            Cursor::Available(Point::new(10.0, 20.0))
        );
        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(Point::new(10.0, 20.0))
        );
        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(Point::new(10.0, 20.0))
        );

        let res_crop = cr.get_crop_rec().unwrap();

        assert_eq!(
            Rectangle::new(
                Point::new(10.0, 20.0),
                min_crop_size
            ),
            res_crop
        );
    }

    #[test]
    fn scaled() {
        let crop_bounds = Rectangle::new(
            Point::new(0.0, 0.0),
            Size::new(100.0, 200.0),
        );
        let bounds = Rectangle::new(
            Point::new(0.0, 0.0),
            Size::new(100.0 / 2.0, 200.0 / 2.0),
        );

        let min_crop_size = Size::new(10.0, 10.0);

        let mut state = State::default();
        let cr = CropTool::new(crop_bounds, min_crop_size);

        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(10.0, 20.0) }),
            bounds,
            Cursor::Available(Point::new(10.0, 20.0))
        );
        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(Point::new(10.0, 20.0))
        );
        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(30.0, 40.0) }),
            bounds,
            Cursor::Available(Point::new(30.0, 40.0))
        );
        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(Point::new(30.0, 40.0))
        );

        let res_crop = cr.get_crop_rec().unwrap();

        assert_eq!(
            Rectangle::new(
                Point::new(20.0, 40.0),
                Size::new(40.0, 40.0)
            ),
            res_crop
        );
    }

    #[test]
    fn move_bottom_right() {
        let crop_bounds = Rectangle::new(
            Point::new(0.0, 0.0),
            Size::new(100.0, 200.0),
        );
        let bounds = Rectangle::new(
            Point::new(0.0, 0.0),
            Size::new(100.0 / 2.0, 200.0 / 2.0),
        );

        let min_crop_size = Size::new(10.0, 10.0);

        let mut state = State::default();
        let cr = CropTool::new(crop_bounds, min_crop_size);

        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(10.0, 20.0) }),
            bounds,
            Cursor::Available(Point::new(10.0, 20.0))
        );
        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(Point::new(10.0, 20.0))
        );
        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(30.0, 40.0) }),
            bounds,
            Cursor::Available(Point::new(30.0, 40.0))
        );
        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(Point::new(30.0, 40.0))
        );
        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(Point::new(11.0, 39.0))
        );
        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(49.0, 99.0) }),
            bounds,
            Cursor::Available(Point::new(49.0, 99.0))
        );
        <CropTool as Program<Message>>::update(
            &cr,
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            bounds,
            Cursor::Available(Point::new(49.0, 99.0))
        );

        let res_crop = cr.get_crop_rec().unwrap();

        assert_eq!(
            Rectangle::new(
                Point::new(60.0, 160.0),
                Size::new(40.0, 40.0)
            ),
            res_crop
        );
    }
}
