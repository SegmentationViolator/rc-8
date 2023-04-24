use clap::Parser;

#[derive(Parser)]
#[command(about, author, version)]
struct Options {
    /// Run in debugger mode
    #[arg(long = "debugger")]
    debug_mode: bool,

    /// Wrap the sprites drawn beyond the edge of the screen, (clips/crops them by default)
    #[arg(long)]
    wrap_sprites: bool,
}

fn main() {
    let options = Options::parse();

    eframe::run_native(
        "RC-8",
        eframe::NativeOptions {
            drag_and_drop_support: false,
            run_and_return: false,
            ..Default::default()
        },
        Box::new(move |cc| Box::new(rc_8::ui::App::new(cc, rc_8::frontend::Options { debug_mode: options.debug_mode, wrap_sprites: options.wrap_sprites, ..Default::default() }))),
    );
}
