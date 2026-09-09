use std::{env, error::Error, sync::Arc};
use systems_modeler_core::Project;
use systems_modeler_persistence::ProjectDatabase;
use systems_modeler_server::{Config, Service, serve, token_hash};
use uuid::Uuid;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("credential") if args.len() == 2 => {
            let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
            println!("{}", serde_json::json!({"actor":Uuid::new_v4(),"token":token,"token_sha256":token_hash(&token)}));
        }
        Some("init") if args.len() == 4 => {
            let mut database = ProjectDatabase::open(&args[2])?;
            let project = Project::new(&args[3]);
            database.save_project(&project)?;
            println!("{}", project.id);
        }
        Some("serve") if args.len() == 4 => {
            if std::fs::metadata(&args[3])?.len() > 1024 * 1024 { return Err("configuration too large".into()); }
            let config: Config = serde_json::from_slice(&std::fs::read(&args[3])?)?;
            let service = Arc::new(Service::new(ProjectDatabase::open(&args[2])?, config)?);
            let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:4783").await?;
                eprintln!("Collaboration API listening on 127.0.0.1:4783; HTTPS proxy required for remote clients.");
                serve(listener, service).await
            })?;
        }
        _ => return Err("usage: systems-modeler-server credential | init DATABASE NAME | serve DATABASE CONFIG".into()),
    }
    Ok(())
}
