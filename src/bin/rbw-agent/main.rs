use anyhow::Context as _;

mod actions;
mod agent;
mod daemon;
mod sock;

async fn tokio_main(
    startup_ack: crate::daemon::StartupAck,
) -> anyhow::Result<()> {
    let listener =
        crate::sock::listen().context("failed to listen on socket")?;

    startup_ack.ack()?;

    let mut agent = crate::agent::Agent::new()?;
    agent.run(listener).await?;

    Ok(())
}

fn real_main() -> anyhow::Result<()> {
    env_logger::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    let startup_ack = daemon::daemonize().context("failed to daemonize")?;

    let (w, r) = std::sync::mpsc::channel();
    // can't use tokio::main because we need to daemonize before starting the
    // tokio runloop, or else things break
    // unwrap is fine here because there's no good reason that this should
    // ever fail
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        if let Err(e) = tokio_main(startup_ack).await {
            // this unwrap is fine because it's the only real option here
            w.send(e).unwrap();
        }
    });

    if let Ok(e) = r.recv() {
        return Err(e);
    }

    Ok(())
}

fn main() {
    let res = real_main();

    if let Err(e) = res {
        // XXX log file?
        eprintln!("{:#}", e);
        std::process::exit(1);
    }
}