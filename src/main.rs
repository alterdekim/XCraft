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
use wry::http::{version, Request, Response};
use wry::{RequestAsyncResponder, WebView, WebViewBuilder};

mod config;
mod launcher;
mod util;
mod minecraft;

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

        let (sx, mut dl_rec) = mpsc::unbounded_channel();

        loop {
            if let Some((ui_action, params, responder)) = receiver.recv().await {
                let ui_action = &ui_action[16..];
                match ui_action {
                    "ui" => responder.respond(Response::new(include_bytes!("www/portable.html"))),
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
                                launcher.load_config();
                                responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["show_add".to_string(), "sidebar_on".to_string()] }).unwrap()))
                            }
                        }
                    }
                    "sign_up" => {
                        let user_name = params.as_ref().unwrap().params.get(0).unwrap();
                        launcher.init_config(user_name.to_string());
                        responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["show_add".to_string(), "sidebar_on".to_string()] }).unwrap()));
                    }
                    "fetch_official_versions" => {
                        if let Ok(versions) = crate::minecraft::versions::fetch_versions_list().await {
                            let versions: Vec<String> = versions.versions.iter().map(|t| t.id.clone()).collect();
                            responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: [ vec!["set_downloadable_versions".to_string()], versions ].concat() }).unwrap()));
                        } else {
                            responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: Vec::new() }).unwrap()));
                        }
                    }
                    "download_vanilla" => {
                        let version = params.unwrap().params[0].clone();
                        println!("Version: {}", version);
                        if let Ok(versions) = crate::minecraft::versions::fetch_versions_list().await {
                            println!("Got versions");
                            let version = versions.versions.iter().find(|t| t.id.clone() == version);
                            if let Some(version) = version {
                                println!("Found");
                                match crate::minecraft::versions::fetch_version_object(version).await {
                                    Ok(config ) => {
                                        println!("Config: {}", config.id);
                                        responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["show_loading".to_string(), "sidebar_off".to_string()] }).unwrap()));
                                        launcher.new_vanilla_instance(config, version, sx.clone()).await;
                                    }
                                    Err(e) => {
                                        println!("Error: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    "fetch_instances_list" => {
                        let resp = launcher.get_instances_list();
                        let mut v: Vec<String> = Vec::new();
                        v.push("set_instances_list".to_string());
                        for (id, release_type, img) in resp {
                            v.push(id);
                            v.push(release_type);
                            v.push(img);
                        }

                        responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: v }).unwrap()));
                    }
                    "check_download_status" => {
                        if let Ok((percent, text)) = dl_rec.try_recv() {
                            responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["update_downloads".to_string(), text, percent.to_string()] }).unwrap()));
                        } else {
                            responder.respond(Response::new(vec![]));
                        }
                    }
                    "run_instance" => {
                        let instance_name = params.unwrap().params[0].clone();
                        launcher.launch_instance(instance_name).await;
                    }
                    "locate_java" => {
                        if let Ok(java_path) = java_locator::locate_file("java.exe") {
                            launcher.config.java_path = java_path.clone();
                            responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["locate_java".to_string(), [&java_path, "java.exe"].join("\\")] }).unwrap()));
                        } else {
                            // todo: implement error notifications
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    event_loop.run_app(&mut app).unwrap();
}