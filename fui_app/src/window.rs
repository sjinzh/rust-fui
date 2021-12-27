use crate::{DrawingContext, FuiDrawingContext, GlWindow, WindowOptions};
use anyhow::Result;
use drawing_gl::GlRenderTarget;
use fui_core::ControlObject;
use fui_core::Rect;
use fui_core::Size;
use fui_core::ViewModel;
use fui_core::WindowService;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

enum WindowState {
    BeforeCreation {
        window_options: WindowOptions,
        view: Option<Rc<RefCell<dyn ControlObject>>>,
    },

    Created {
        core_window: fui_core::Window<GlWindow>,
        _view: Option<Rc<RefCell<dyn ControlObject>>>,
        _window_services: Rc<RefCell<fui_core::Services>>,
    },
}

#[derive(Clone)]
pub struct Window {
    state: Rc<RefCell<WindowState>>,
}

impl Window {
    pub fn new(window_options: WindowOptions) -> Self {
        Self {
            state: Rc::new(RefCell::new(WindowState::BeforeCreation {
                window_options,
                view: None,
            })),
        }
    }

    pub fn downgrade(&self) -> WindowWeak {
        WindowWeak {
            state: Rc::downgrade(&self.state),
        }
    }

    pub fn set_vm<V: ViewModel>(&self, view_model: Rc<RefCell<V>>) {
        let new_view = ViewModel::create_view(&view_model);

        match &mut *self.state.borrow_mut() {
            WindowState::BeforeCreation { ref mut view, .. } => {
                view.replace(new_view);
            }
            WindowState::Created {
                ref mut core_window,
                ..
            } => core_window.add_layer(new_view),
        }
    }

    pub fn create(&mut self, drawing_context: &Rc<RefCell<DrawingContext>>) -> Result<()> {
        let mut native_window = fui_system::Window::new(None)?;

        let created = match &mut *self.state.borrow_mut() {
            WindowState::BeforeCreation {
                ref mut window_options,
                ref mut view,
                ..
            } => {
                native_window.set_title(&window_options.title)?;
                native_window.resize(window_options.width, window_options.height);
                native_window.set_visible(window_options.visible)?;
                if window_options.icon.len() > 0 {
                    let icon = fui_system::Icon::from_data(&window_options.icon)?;
                    native_window.set_icon(&icon)?;
                }

                let window_service_rc: Rc<RefCell<dyn WindowService>> =
                    self.get_window_service().clone();

                let services = Rc::new(RefCell::new(fui_core::Services::new(&window_service_rc)));

                let mut core_window = fui_core::Window::new(GlWindow::new(native_window));
                let mut view_clone = None;
                if let Some(view) = view {
                    view_clone = Some(view.clone());
                    core_window.add_layer(view.clone());
                }
                self.setup_window(&mut core_window, &drawing_context, Rc::downgrade(&services));

                WindowState::Created {
                    core_window,
                    _view: view_clone,
                    _window_services: services,
                }
            }
            WindowState::Created { .. } => panic!("create() called on already created window!"),
        };

        *self.state.borrow_mut() = created;

        Ok(())
    }

    pub fn get_window_service(&self) -> Rc<RefCell<dyn fui_core::WindowService + 'static>> {
        let service: Rc<RefCell<dyn fui_core::WindowService + 'static>> = self.state.clone();
        service
    }

    pub fn setup_window(
        &self,
        core_window: &mut fui_core::Window<GlWindow>,
        drawing_context: &Rc<RefCell<DrawingContext>>,
        services: Weak<RefCell<fui_core::Services>>,
    ) {
        core_window
            .get_root_control()
            .borrow_mut()
            .get_context_mut()
            .set_services(Some(services));

        core_window.native_window.window.on_paint_gl({
            let window_weak = self.downgrade();
            let drawing_context_clone = drawing_context.clone();
            let mut initialized = false;

            move || {
                if let Some(window) = window_weak.upgrade() {
                    if let WindowState::Created { core_window, .. } =
                        &mut *window.state.borrow_mut()
                    {
                        let mut drawing_context = drawing_context_clone.borrow_mut();

                        if !initialized {
                            core_window.native_window.gl_context_data =
                                Some(drawing_context.device.init_context(|symbol| {
                                    core_window
                                        .native_window
                                        .window
                                        .get_opengl_proc_address(symbol)
                                        .unwrap()
                                }));
                            initialized = true;
                        }

                        let width = core_window.native_window.window.get_width();
                        let height = core_window.native_window.window.get_height();
                        if width > 0 && height > 0 {
                            Self::update_min_window_size(core_window, &mut drawing_context, 0);

                            Self::render(
                                core_window,
                                &mut drawing_context,
                                width as u32,
                                height as u32,
                                0,
                            );
                        }
                    }
                }
            }
        });

        core_window.native_window.window.on_event({
            let window_weak = self.downgrade();
            let drawing_context_clone = drawing_context.clone();

            move |event| {
                if let Some(window) = window_weak.upgrade() {
                    if let Some(input_event) = crate::event_converter::convert_event(&event) {
                        if let WindowState::Created { core_window, .. } =
                            &mut *window.state.borrow_mut()
                        {
                            let mut drawing_context = drawing_context_clone.borrow_mut();

                            let width = core_window.native_window.window.get_width();
                            let height = core_window.native_window.window.get_height();
                            let mut fui_drawing_context = FuiDrawingContext::new(
                                (width as u16, height as u16),
                                &mut drawing_context,
                                0,
                            );

                            // events go to the window's root control
                            let root_view = core_window.get_root_control().clone();
                            core_window.event_processor.handle_event(
                                &root_view,
                                &mut fui_drawing_context,
                                &input_event,
                            );

                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        });
    }

    fn is_dirty(window: &mut fui_core::Window<GlWindow>) -> bool {
        window.get_root_control().borrow().get_context().is_dirty()
    }

    fn update_min_window_size(
        window: &mut fui_core::Window<GlWindow>,
        drawing_context: &mut DrawingContext,
        background_texture: i32,
    ) {
        let size = Size::new(0.0f32, 0.0f32);

        let mut fui_drawing_context = FuiDrawingContext::new(
            (size.width as u16, size.height as u16),
            drawing_context,
            background_texture,
        );

        let min_size = {
            let mut root_control = window.get_root_control().borrow_mut();
            root_control.measure(&mut fui_drawing_context, size);
            root_control.get_rect()
        };

        window
            .native_window
            .window
            .set_minimum_size(min_size.width as i32, min_size.height as i32);
    }

    pub fn render(
        window: &mut fui_core::Window<GlWindow>,
        drawing_context: &mut DrawingContext,
        width: u32,
        height: u32,
        background_texture: i32,
    ) {
        let size = Size::new(width as f32, height as f32);

        let mut fui_drawing_context = FuiDrawingContext::new(
            (size.width as u16, size.height as u16),
            drawing_context,
            background_texture,
        );

        let mut primitives = Vec::new();

        // background texture
        primitives.push(drawing::primitive::Primitive::Image {
            resource_key: background_texture,
            rect: drawing::units::PixelRect::new(
                drawing::units::PixelPoint::new(0.0f32, 0.0f32),
                drawing::units::PixelSize::new(size.width, size.height),
            ),
            uv: [
                0.0f32,
                0.0f32,
                1.0f32 * size.width / 256.0f32,
                1.0f32 * size.height / 256.0f32,
            ],
        });

        {
            let mut root_control = window.get_root_control().borrow_mut();

            root_control.measure(&mut fui_drawing_context, size);
            root_control.set_rect(
                &mut fui_drawing_context,
                Rect::new(0f32, 0f32, size.width, size.height),
            );

            let (mut primitives1, mut overlay) =
                root_control.to_primitives(&mut fui_drawing_context);
            primitives.append(&mut primitives1);
            primitives.append(&mut overlay);

            root_control.get_context_mut().set_is_dirty(false);
        }

        let res = drawing_context.begin(&mut window.native_window);
        if let Err(err) = res {
            eprintln!("Render error on begin drawing: {}", err);
        } else {
            let render_target = GlRenderTarget::new(0, width as u16, height as u16, 1.0f32);

            drawing_context.clear(&render_target, &[0.3f32, 0.4f32, 0.3f32, 1.0f32]);
            let res = drawing_context.draw(&render_target, &primitives);
            if let Err(err) = res {
                eprintln!("Render error: {}", err);
            }
            drawing_context.end(&mut window.native_window);
        }
    }
}

impl fui_core::WindowService for WindowState {
    fn add_layer(&mut self, control: Rc<RefCell<dyn ControlObject>>) {
        match self {
            WindowState::BeforeCreation { .. } => (),
            WindowState::Created { core_window, .. } => {
                core_window.add_layer(control);
            }
        }
    }

    fn remove_layer(&mut self, control: &Rc<RefCell<dyn ControlObject>>) {
        match self {
            WindowState::BeforeCreation { .. } => (),
            WindowState::Created { core_window, .. } => {
                core_window.remove_layer(control);
            }
        }
    }

    fn repaint(&mut self) {
        match self {
            WindowState::BeforeCreation { .. } => (),
            WindowState::Created { core_window, .. } => {
                core_window.repaint();
            }
        }
    }
}

pub struct WindowWeak {
    state: Weak<RefCell<WindowState>>,
}

impl WindowWeak {
    pub fn upgrade(&self) -> Option<Window> {
        self.state.upgrade().map(|s| Window { state: s })
    }
}
