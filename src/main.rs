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
use wry::http::Request;
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
}

impl ApplicationHandler for App {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let window = event_loop.create_window(Window::default_attributes().with_inner_size(LogicalSize::new(900, 600)).with_min_inner_size(LogicalSize::new(900, 600)).with_title("XCraft")).unwrap();
    let webview = WebViewBuilder::new()
      .with_html(include_str!("www/portable.html"))
      .with_ipc_handler(|request| {
        SENDER.lock().unwrap().as_ref().unwrap().send(request);
      })  
      .build(&window)
      .unwrap();

    

    self.window = Some(window);

    self.launcher_start(Arc::new(webview));
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

impl App {
  fn launcher_start(&mut self, webview: Arc<WebView>) {
    tokio::spawn(async move {
      let (sender_gui, mut receiver_gui) = mpsc::unbounded_channel();
      *SENDERGUI.lock().unwrap() = Some(sender_gui);
      loop {
        if let Some(action) = receiver_gui.recv().await {
            println!("YUPPIE");
            webview.evaluate_script("alert('done')");
        }
      }
    });
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