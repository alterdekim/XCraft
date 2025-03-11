use winit::application::ApplicationHandler;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::event::{Event, WindowEvent};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::{Window, WindowId};
use winit::event_loop::ActiveEventLoop;
use wry::WebViewBuilder;

#[derive(Default)]
struct App {
  window: Option<Window>,
  webview: Option<wry::WebView>,
}

impl ApplicationHandler for App {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let window = event_loop.create_window(Window::default_attributes()).unwrap();
    let webview = WebViewBuilder::new()
      .with_html(include_str!("www/sign_in.html"))
      .build(&window)
      .unwrap();

    self.window = Some(window);
    self.webview = Some(webview);
  }

  fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
  }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}