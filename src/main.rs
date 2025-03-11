use web_view::Content;

fn main() {
    let html_content = String::from_utf8(std::fs::read("main_screen").unwrap()).unwrap();
	
    web_view::builder()
        .title("My Project")
        .content(Content::Html(html_content))
        .size(800, 600)
        .resizable(false)
        .debug(true)
        .user_data(())
        .invoke_handler(|_webview, _arg| Ok(()))
        .run()
        .unwrap();
}
