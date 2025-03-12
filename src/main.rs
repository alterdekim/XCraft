use std::sync::{Arc, Mutex};

use config::LauncherConfig;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use winit::application::ApplicationHandler;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::event::{Event, WindowEvent};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::{Window, WindowId};
use winit::event_loop::ActiveEventLoop;
use wry::dpi::LogicalSize;
use wry::http::{Request, Response};
use wry::{WebView, WebViewBuilder, WebViewBuilderExtWindows};

mod config;

static SENDER: Mutex<Option<UnboundedSender<Request<String>>>> = Mutex::new(None);
static SENDERGUI: Mutex<Option<UnboundedSender<UIAction>>> = Mutex::new(None);

enum UIAction {
  DoSomething
}

#[derive(Default)]
struct App {
  window: Option<Window>,
  webview: Option<WebView>
}

impl ApplicationHandler for App {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let window = event_loop.create_window(Window::default_attributes().with_inner_size(LogicalSize::new(900, 600)).with_min_inner_size(LogicalSize::new(900, 600)).with_title("XCraft")).unwrap();
    let webview = WebViewBuilder::new()
      .with_html(include_str!("www/portable.html"))
      .with_asynchronous_custom_protocol("xcraft".into(), move |wid, request, responder| {
          let uri = request.uri().to_string();
          println!("GOTCHA");
          let response = "yeeah!".as_bytes();
          tokio::spawn(async move {
            responder.respond(Response::new(response));
          });
      })  
      .build(&window)
      .unwrap();

    

    self.window = Some(window);
    self.webview = Some(webview);
  }

  fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            _ => {}
        }
  }
}

#[tokio::main]
async fn main() {
  let (snd, mut receiver) = mpsc::unbounded_channel();

    *SENDER.lock().unwrap() = Some(snd);

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();

    let rt = Runtime::new().unwrap();

    rt.spawn(async move {
        loop {
          if let Some(request) = receiver.recv().await {
              println!("Request: {}", request.body());
              SENDERGUI.lock().unwrap().as_ref().unwrap().send(UIAction::DoSomething);
          }
        }
    });

    event_loop.run_app(&mut app).unwrap();
}