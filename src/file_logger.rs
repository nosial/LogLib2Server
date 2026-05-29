use crate::event::LogEvent;
use chrono::Local;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct FileLogger {
    output_dir: PathBuf,
    current_date: Mutex<String>,
    writer: Mutex<Option<BufWriter<fs::File>>>,
}

impl FileLogger {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> std::io::Result<Self> {
        let dir = output_dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        Ok(Self {
            output_dir: dir,
            current_date: Mutex::new(String::new()),
            writer: Mutex::new(None),
        })
    }

    pub fn write_event(&self, event: &LogEvent) -> std::io::Result<()> {
        let today = Local::now().format("%Y-%m-%d").to_string();

        let mut current_date = self.current_date.lock().unwrap();
        let mut writer_opt = self.writer.lock().unwrap();

        if *current_date != today || writer_opt.is_none() {
            if let Some(ref mut w) = *writer_opt {
                let _ = w.flush();
            }

            let file_path = self.output_dir.join(format!("log{today}.jsonl"));
            let file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)?;

            *writer_opt = Some(BufWriter::new(file));
            *current_date = today;
        }
        drop(current_date);

        if let Some(ref mut writer) = *writer_opt {
            let line = serde_json::to_string(event)?;
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
        drop(writer_opt);

        Ok(())
    }
}
