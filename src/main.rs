use std::sync::{Arc, Mutex};

use config::LauncherConfig;
use launcher::Launcher;
use serde::{Deserialize, Serialize};
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
use wry::{RequestAsyncResponder, WebView, WebViewBuilder, WebViewBuilderExtWindows};

mod config;
mod launcher;
mod util;

static SENDER: Mutex<Option<UnboundedSender<(String, Option<UIMessage>, RequestAsyncResponder)>>> = Mutex::new(None);

#[derive(Serialize, Deserialize)]
struct UIMessage {
    params: Vec<String>
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
      .with_asynchronous_custom_protocol("xcraft".into(), move |wid, request, responder| {
          let uri = request.uri().to_string();
          if let Ok(msg) = serde_json::from_slice(request.body()) {
            let _ = SENDER.lock().unwrap().as_ref().unwrap().send((uri, Some(msg), responder));
            return;
          }
          let _ = SENDER.lock().unwrap().as_ref().unwrap().send((uri, None, responder));
      })
      .with_url("xcraft://custom/ui")  
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
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();

    let rt = Runtime::new().unwrap();

    rt.spawn(async move {
        let (snd, mut receiver) = mpsc::unbounded_channel();
        *SENDER.lock().unwrap() = Some(snd);

        let mut launcher = Launcher::default();

        loop {
            if let Some((ui_action, params, responder)) = receiver.recv().await {
               println!("Command: {}", ui_action);
               println!("params: {}", params.is_some());
               
                let ui_action = &ui_action[16..];
                match ui_action {
                    "ui" => responder.respond(Response::new(include_str!("www/portable.html").as_bytes())),
                    "portable" => {
                        launcher.config.set_portable(true);
                        launcher.init_dirs();
                        responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["show_login".to_string()] }).unwrap()))
                    }
                    "installation" => {
                        launcher.init_dirs();
                        responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["show_login".to_string()] }).unwrap()))
                    }
                    "check_installation" => {
                        if launcher.is_portable() {
                            launcher.config.set_portable(true);
                            launcher.init_dirs();
                            if !launcher.is_config_exist() {
                                responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["show_login".to_string()] }).unwrap()))
                            } else {
                                responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["show_add".to_string(), "sidebar_on".to_string()] }).unwrap()))
                            }
                        }
                    }
                    "sign_up" => {
                        let user_name = params.as_ref().unwrap().params.get(0).unwrap();
                        launcher.init_config(user_name.to_string());
                        responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["show_add".to_string(), "sidebar_on".to_string()] }).unwrap()));
                    }
                    _ => {}
                }
            }
        }
    });

    event_loop.run_app(&mut app).unwrap();
}