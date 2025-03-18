use std::sync::{Arc, Mutex};

use base64::prelude::{BASE64_STANDARD, BASE64_STANDARD_NO_PAD};
use base64::Engine;
use config::LauncherConfig;
use launcher::Launcher;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
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
        let (mut lx, mut logs_rec) = mpsc::unbounded_channel();

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
                            let versions: Vec<String> = versions.versions.iter().filter(|t| {
                                if !launcher.config.show_alpha && t.r#type == "old_alpha" {
                                    return false;
                                } else if !launcher.config.show_beta && t.r#type == "old_beta" {
                                    return false;
                                } else if !launcher.config.show_snapshots && t.r#type == "snapshot" {
                                    return false;
                                }
                                return true;
                            }).map(|t| t.id.clone()).collect();
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
                    "fetch_servers_list" => {
                        let resp = launcher.get_servers_list().await;
                        let mut v: Vec<String> = Vec::new();
                        v.push("fetch_servers_list".to_string());
                        for (domain, nickname, image) in resp {
                            v.push(domain);
                            v.push(nickname);
                            v.push(image.unwrap_or(String::new()));
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
                    "check_logs_status" => {
                        if let Ok(text) = logs_rec.try_recv() {
                            responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["update_logs".to_string(), text] }).unwrap()));
                        } else {
                            responder.respond(Response::new(vec![]));
                        }
                    }
                    "run_instance" => {
                        let instance_name = params.unwrap().params[0].clone();
                        logs_rec.close();
                        (lx, logs_rec) = mpsc::unbounded_channel();
                        launcher.launch_instance(instance_name, launcher.config.user_name().to_string(), util::random_string(32), util::random_string(32), lx.clone()).await;
                    }
                    "locate_java" => {
                        if let Ok(java_path) = java_locator::locate_file("java.exe") {
                            launcher.config.java_path = java_path.clone();
                            responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["locate_java".to_string(), [&java_path, "java.exe"].join("\\")] }).unwrap()));
                        } else {
                            // todo: implement error notifications
                        }
                    }
                    "add_server" => {
                        let params = &params.unwrap().params;
                        let (status, msg) = launcher.register_user_server(params[0].clone(), params[1].clone(), params[2].clone()).await;
                        responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["add_server_response".to_string(), status.to_string(), msg.to_string()] }).unwrap()));
                    }
                    "fetch_settings" => {
                        responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["fetch_settings_response".to_string(), launcher.config.show_alpha.to_string(), launcher.config.show_beta.to_string(), launcher.config.show_snapshots.to_string(), launcher.config.java_path.clone(), launcher.config.ram_amount.to_string(), launcher.config.enable_blur.to_string()] }).unwrap()));
                    }
                    "save_bg" => {
                        let params = &params.unwrap().params;
                        let mut p = launcher.config.launcher_dir();
                        p.push("bg.base64");
                        let _ = std::fs::write(p, &params[0]);
                    }
                    "fetch_bg" => {
                        let mut p = launcher.config.launcher_dir();
                        p.push("bg.base64");
                        if let Ok(data) = std::fs::read(p) {
                            responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: vec!["fetch_bg".to_string(), String::from_utf8(data).unwrap()] }).unwrap()));
                        }
                    }
                    "update_settings" => {
                        let params = &params.unwrap().params;
                        launcher.config.java_path = params[3].clone();
                        launcher.config.ram_amount = params[4].parse().unwrap();
                        launcher.config.show_alpha = params[0].parse().unwrap();
                        launcher.config.show_beta = params[1].parse().unwrap();
                        launcher.config.show_snapshots = params[2].parse().unwrap();
                        launcher.config.enable_blur = params[5].parse().unwrap();
                        launcher.save_config();
                    }
                    "open_file" => {
                        let params = &params.unwrap().params;
                        let _ = Command::new("cmd").args(["/C", "start", &params[0]]).spawn();
                    }
                    "load_screenshots" => {
                        let screenshots = launcher.get_screenshots();
                        let mut svec = Vec::new();
                        for (path,data) in screenshots {
                            svec.push(path);
                            svec.push(data);
                        }
                        responder.respond(Response::new(serde_json::to_vec(&UIMessage { params: [vec!["load_screenshots".to_string()], svec].concat() }).unwrap()));
                    }
                    _ => {}
                }
            }
        }
    });

    event_loop.run_app(&mut app).unwrap();
}