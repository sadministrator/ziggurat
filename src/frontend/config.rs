use std::fs::{self, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;

#[derive(Debug)]
struct Config {
    config: String,
}

impl Config {
    fn load_config<P: AsRef<Path>>(file_path: P) -> io::Result<Self> {
        let file_path = file_path.as_ref();
        if file_path.exists() {
            let config = fs::read_to_string(file_path)?;
            Ok(Config { app_config: config })
        } else {
            let default_config = "default_ziggurat_config\n";
            fs::write(file_path, default_config)?;
            Ok(Config {
                app_config: default_config,
            })
        }
    }

    fn update_config<P: AsRef<Path>>(&mut self, file_path: P) -> io::Result<()> {
        let file_path = file_path.as_ref();
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut writer = BufWriter::new(
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(file_path)?,
        );
        loop {
            let mut buffer = String::new();
            reader.read_line(&mut buffer)?;
            self.app_config = buffer.trim().to_string();
            writer.write_all(self.app_config.as_bytes())?;
            writer.flush()?;
        }
    }
}

struct ConfigUpdater {
    config: Config,
    file_path: String,
}

impl ConfigUpdater {
    fn new<P: AsRef<Path>>(file_path: P) -> io::Result<Self> {
        let file_path = file_path.as_ref().to_string_lossy().into_owned();
        let config = Config::load_config(file_path)?;
        Ok(ConfigUpdater { config, file_path })
    }

    fn update(&mut self) -> io::Result<()> {
        self.config.update_config(self.file_path.clone())?;
        Ok(())
    }
}
