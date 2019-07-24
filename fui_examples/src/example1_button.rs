#![windows_subsystem = "windows"]

extern crate fui;
extern crate fui_macros;
extern crate typed_builder;
extern crate typemap;
extern crate winit;

use fui::application::*;
use fui::controls::*;
use fui::layout::*;
use fui::*;
use fui_macros::ui;

use std::cell::RefCell;
use std::rc::Rc;

use typed_builder::TypedBuilder;
use typemap::TypeMap;

struct MainViewModel {
    pub counter: Property<i32>,
    pub counter2: Property<i32>,
}

impl MainViewModel {
    pub fn new() -> Self {
        MainViewModel {
            counter: Property::new(10),
            counter2: Property::new(0),
        }
    }

    pub fn increase(&mut self) {
        self.counter.change(|c| c + 1);
    }

    pub fn decrease(&mut self) {
        self.counter.change(|c| c - 1);
    }
}

#[derive(TypedBuilder)]
pub struct ButtonText {
    #[builder(default_code = "Property::new(\"\".to_string())")]
    pub text: Property<String>,
    #[builder(default_code = "Callback::empty()")]
    pub clicked: Callback<()>,
}

impl View for ButtonText {
    fn to_view(self, _context: ViewContext) -> Rc<RefCell<ControlObject>> {
        ui! {
            Button {
                clicked: self.clicked,
                Text { text: self.text }
            }
        }
    }
}

impl View for MainViewModel {
    fn to_view(self, _context: ViewContext) -> Rc<RefCell<ControlObject>> {
        let view_model = &Rc::new(RefCell::new(self));
        let vm: &mut MainViewModel = &mut view_model.borrow_mut();

        vm.counter2.bind(&mut vm.counter);
        vm.counter.bind(&mut vm.counter2);

        ui!(
            Horizontal {
                Text { text: (&vm.counter, |counter| format!("Counter {}", counter)) },
                Button {
                    clicked: Callback::new(view_model, |vm, _| vm.decrease()),
                    Text { text: "Decrease" }
                },
                ButtonText {
                    clicked: Callback::new(view_model, |vm, _| vm.increase()),
                    text: "Increase"
                },
                Text { text: (&vm.counter2, |counter| format!("Counter2 {}", counter)) },
            }
        )
    }
}

fn main() {
    let mut app = Application::new("Example: button").unwrap();

    let main_view_model = MainViewModel::new();

    {
        let mut window_manager = app.get_window_manager().borrow_mut();
        let window_builder = winit::WindowBuilder::new().with_title("Example: button");
        window_manager
            .add_window_view_model(window_builder, app.get_events_loop(), main_view_model)
            .unwrap();
    }

    app.run();
}
