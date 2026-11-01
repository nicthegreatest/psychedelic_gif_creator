/// The main window. Empty for now: a header bar over an empty content area.
pub fn build(app: &adw::Application) -> adw::ApplicationWindow {
    let content = adw::ToolbarView::new();
    content.add_top_bar(&adw::HeaderBar::new());

    adw::ApplicationWindow::builder()
        .application(app)
        .title("Psychedelic GIF Creator")
        .default_width(1024)
        .default_height(768)
        .content(&content)
        .build()
}
