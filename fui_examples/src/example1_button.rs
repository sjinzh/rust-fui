#![windows_subsystem = "windows"]

extern crate fui;
extern crate fui_macros;
extern crate winit;

use fui::application::*;
use fui::controls::*;
use fui::layout::*;
use fui::*;
use fui_macros::ui;

use std::cell::RefCell;
use std::rc::Rc;

use Property;

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

impl View for MainViewModel {
    fn create_view(view_model: &Rc<RefCell<MainViewModel>>) -> ViewData {
        let mut bindings = Vec::<fui::EventSubscription>::new();

        let root_control = ui!(
            Horizontal {
                Text { text: {
                    let mut prop = Property::new("".to_string());
                    let vm: &mut MainViewModel = &mut view_model.borrow_mut();
                    let binding = prop.bind_c(&mut vm.counter, |counter| format!("Counter {}", counter));
                    bindings.push(binding);
                    prop
                } },
                Button {
                    clicked: Callback::new(view_model, |vm, _| vm.decrease()),
                    Text { text: Property::new("Decrease".to_string()) }
                },
                Button {
                    clicked: Callback::new(view_model, |vm, _| vm.increase()),
                    Text { text: Property::new("Increase".to_string()) }
                },
                Text { text: {
                    let mut prop = Property::new("".to_string());
                    let vm: &mut MainViewModel = &mut view_model.borrow_mut();
                    let binding = prop.bind_c(&mut vm.counter2, |counter| format!("Counter2 {}", counter));
                    bindings.push(binding);
                    prop
                 } },
            }
        );

        let vm: &mut MainViewModel = &mut view_model.borrow_mut();
        bindings.push(vm.counter2.bind(&mut vm.counter));
        bindings.push(vm.counter.bind(&mut vm.counter2));

        ViewData {
            root_control: root_control,
            bindings: bindings,
        }
    }
}

fn main() {
    let mut app = Application::new("Marek Ogarek").unwrap();

    let main_view_model = Rc::new(RefCell::new(MainViewModel::new()));

    {
        let mut window_manager = app.get_window_manager().borrow_mut();
        let window_builder = winit::WindowBuilder::new().with_title("Window 1");
        window_manager
            .add_window_view_model(window_builder, app.get_events_loop(), &main_view_model)
            .unwrap();
    }

    app.run();
}
