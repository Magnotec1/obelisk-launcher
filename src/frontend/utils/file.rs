use relm4::gtk;

pub fn open_instance_subfolder(base_dir: &std::path::Path, subfolder: &str) {
    let target_dir = base_dir.join(subfolder);
    if !target_dir.exists() {
        let _ = std::fs::create_dir_all(&target_dir);
    }

    let file = gtk::gio::File::for_path(&target_dir);
    let launcher = gtk::FileLauncher::new(Some(&file));
    launcher.launch(None::<&gtk::Window>, None::<&gtk::gio::Cancellable>, move |res| {
        if let Err(e) = res {
            eprintln!("FileLauncher failed to open {:?}: {}. Falling back to xdg-open.", target_dir, e);
            let _ = std::process::Command::new("xdg-open")
                .arg(&target_dir)
                .spawn();
        }
    });
}

pub fn open_url(url: &str) {
    if url.is_empty() {
        return;
    }
    let launcher = gtk::UriLauncher::new(url);
    launcher.launch(None::<&gtk::Window>, None::<&gtk::gio::Cancellable>, {
        let url_clone = url.to_string();
        move |res| {
            if let Err(e) = res {
                eprintln!("UriLauncher failed to open '{}': {}. Falling back to launch_default_for_uri.", url_clone, e);
                // Fallback to GIO AppInfo
                let launched = gtk::gio::AppInfo::launch_default_for_uri(&url_clone, None::<&gtk::gio::AppLaunchContext>).is_ok();
                if !launched {
                    eprintln!("launch_default_for_uri failed. Falling back to xdg-open.");
                    let _ = std::process::Command::new("xdg-open")
                        .arg(&url_clone)
                        .spawn();
                }
            }
        }
    });
}
