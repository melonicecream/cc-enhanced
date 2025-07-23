use anyhow::Result;

mod app;
mod claude;
mod claude_legacy;
mod features;
mod ide;
mod shared;
mod ui;
mod widgets;

#[cfg(test)]
mod widgets_tests;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the application
    let mut app = app::App::new().await?;

    // Run the TUI
    app.run().await?;

    Ok(())
}
