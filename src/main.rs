use resonant::App;

fn setup_logger() -> Result<(), fern::InitError> {
    let colors = fern::colors::ColoredLevelConfig::new();
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{color}[{timestamp}] [{level}] [{target}] {message}\x1b[0m",
                color = format_args!("\x1b[{}m", colors.get_color(&record.level()).to_fg_str(),),
                timestamp = humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
                level = record.level(),
                target = record.target(),
                message = message,
            ));
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    setup_logger()?;
    iced::application(App::default, App::update, App::view)
        .subscription(App::subscription)
        .run()?;
    Ok(())
}
