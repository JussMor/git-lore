fn resolve(args: ResolveArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.path)?;
    enforce_cli_write_guard(workspace.root(), "edit")?;

    let state = workspace.load_state()?;
    let target_location = args.location.clone();
    
    let candidate_atoms: Vec<LoreAtom> = state.atoms.into_iter()
        .filter(|atom| {
            let p = atom.path.as_ref().map(|v| v.to_string_lossy().replace('\\', "/")).unwrap_or_else(|| "<no-path>".to_string());
            let s = atom.scope.as_deref().unwrap_or("<no-scope>");
            format!("{}::{}", p, s) == target_location && atom.state != AtomState::Deprecated
        })
        .collect();

    if candidate_atoms.is_empty() {
        println!("No active contradictions found at location {target_location}");
        return Ok(());
    }

    let winner_id = if let Some(id) = args.winner_id {
        if !candidate_atoms.iter().any(|a| a.id == id) {
            anyhow::bail!("Atom ID {} not found among active atoms at location {}", id, target_location);
        }
        id
    } else {
        println!("Found {} active atoms at {}:", candidate_atoms.len(), target_location);
        for (i, atom) in candidate_atoms.iter().enumerate() {
            println!("[{}] {} ({:?}) - {}", i, atom.id, atom.state, atom.title);
        }
        
        let idx = loop {
            print!("Select the winning atom index (0-{}): ", candidate_atoms.len() - 1);
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if let Ok(idx) = input.trim().parse::<usize>() {
                if idx < candidate_atoms.len() {
                    break idx;
                }
            }
            println!("Invalid selection.");
        };
        candidate_atoms[idx].id.clone()
    };

    let actor = std::env::var("USER").ok();
    
    for atom in candidate_atoms {
        if atom.id != winner_id {
            // Note: workspace.transition_atom_state requires (id, target_state, reason, actor)
            println!("Deprecating lost atom {}...", atom.id);
            let res = workspace.transition_atom_state(
                &atom.id,
                AtomState::Deprecated,
                args.reason.clone(),
                actor.clone(),
            );
            if let Err(e) = res {
                println!("Warning: failed to deprecate {}: {}", atom.id, e);
            }
        }
    }
    
    // Attempt to accept the winner if it isn't accepted yet
    let winner_atom = workspace.load_state()?.atoms.into_iter().find(|a| a.id == winner_id).unwrap();
    if winner_atom.state != AtomState::Accepted {
        println!("Accepting winning atom {}...", winner_id);
        let res = workspace.transition_atom_state(
            &winner_id,
            AtomState::Accepted,
            args.reason.clone(),
            actor.clone(),
        );
        if let Err(e) = res {
            println!("Warning: failed to accept winner {}: {}", winner_id, e);
        }
    }

    println!("Conflict at {} resolved successfully. [{}] is the winner.", target_location, winner_id);
    Ok(())
}
