// srt_editor.rs
use std::fs;
use std::io::{self, Write, BufRead};
use std::time::Duration;
use std::str::FromStr;
use std::fmt;

#[derive(Debug, Clone)]
struct Subtitle {
    index: usize,
    start: Duration,
    end: Duration,
    text: String,
}

fn parse_time(s: &str) -> Result<Duration, String> {
    let s = s.trim().replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err("Invalid time format".to_string());
    }
    let h: u64 = parts[0].parse().map_err(|_| "Hours")?;
    let m: u64 = parts[1].parse().map_err(|_| "Minutes")?;
    let sec_parts: Vec<&str> = parts[2].split('.').collect();
    let sec: u64 = sec_parts[0].parse().map_err(|_| "Seconds")?;
    let ms: u64 = if sec_parts.len() > 1 {
        sec_parts[1].parse().unwrap_or(0)
    } else { 0 };
    Ok(Duration::from_secs(h*3600 + m*60 + sec) + Duration::from_millis(ms))
}

fn format_time(d: Duration) -> String {
    let total_sec = d.as_secs();
    let h = total_sec / 3600;
    let m = (total_sec % 3600) / 60;
    let s = total_sec % 60;
    let ms = d.subsec_millis();
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
}

fn parse_srt(content: &str) -> Vec<Subtitle> {
    let mut subs = Vec::new();
    let blocks: Vec<&str> = content.trim().split("\n\n").collect();
    for block in blocks {
        let lines: Vec<&str> = block.lines().collect();
        if lines.len() < 3 { continue; }
        let mut idx = 0;
        let mut line_idx = 0;
        if let Ok(num) = lines[0].parse::<usize>() {
            idx = num;
            line_idx = 1;
        }
        if line_idx >= lines.len() { continue; }
        let time_line = lines[line_idx];
        let time_parts: Vec<&str> = time_line.split("-->").collect();
        if time_parts.len() != 2 { continue; }
        let start = parse_time(time_parts[0].trim()).unwrap_or(Duration::from_secs(0));
        let end = parse_time(time_parts[1].trim()).unwrap_or(Duration::from_secs(0));
        let text = lines[line_idx+1..].join("\n");
        subs.push(Subtitle { index: idx, start, end, text });
    }
    for (i, sub) in subs.iter_mut().enumerate() {
        sub.index = i + 1;
    }
    subs
}

fn format_srt(subs: &[Subtitle]) -> String {
    let mut out = String::new();
    for sub in subs {
        out.push_str(&format!("{}\n", sub.index));
        out.push_str(&format!("{} --> {}\n", format_time(sub.start), format_time(sub.end)));
        out.push_str(&sub.text);
        out.push_str("\n\n");
    }
    out
}

struct Editor {
    filename: String,
    subs: Vec<Subtitle>,
}

impl Editor {
    fn load(filename: &str) -> Result<Self, String> {
        let content = fs::read_to_string(filename).map_err(|e| e.to_string())?;
        let subs = parse_srt(&content);
        Ok(Editor { filename: filename.to_string(), subs })
    }
    fn save(&self, filename: Option<&str>) -> Result<(), String> {
        let fname = filename.unwrap_or(&self.filename);
        let content = format_srt(&self.subs);
        fs::write(fname, content).map_err(|e| e.to_string())
    }
    fn shift(&mut self, ms: i64) {
        let delta = if ms >= 0 {
            Duration::from_millis(ms as u64)
        } else {
            Duration::from_millis((-ms) as u64)
        };
        for sub in &mut self.subs {
            if ms >= 0 {
                sub.start += delta;
                sub.end += delta;
            } else {
                if sub.start >= delta { sub.start -= delta; }
                if sub.end >= delta { sub.end -= delta; }
            }
        }
        println!("Сдвиг на {} мс выполнен.", ms);
    }
    fn renumber(&mut self) {
        for (i, sub) in self.subs.iter_mut().enumerate() {
            sub.index = i + 1;
        }
        println!("Перенумерация выполнена.");
    }
    fn validate(&self) -> Vec<String> {
        let mut errs = Vec::new();
        if self.subs.is_empty() {
            errs.push("Нет субтитров.".to_string());
        }
        let mut prev_end = Duration::from_secs(0);
        for (i, sub) in self.subs.iter().enumerate() {
            if sub.start >= sub.end {
                errs.push(format!("#{}: начало >= конец", i+1));
            }
            if i > 0 && sub.start < prev_end {
                errs.push(format!("#{}: перекрытие с предыдущим", i+1));
            }
            prev_end = sub.end;
        }
        errs
    }
    fn list(&self) {
        for sub in &self.subs {
            println!("#{}  {} --> {}\n{}\n", sub.index, format_time(sub.start), format_time(sub.end), sub.text);
        }
    }
    fn add(&mut self, start_str: &str, end_str: &str, text: &str) {
        let start = parse_time(start_str).unwrap_or(Duration::from_secs(0));
        let end = parse_time(end_str).unwrap_or(Duration::from_secs(0));
        let sub = Subtitle { index: self.subs.len()+1, start, end, text: text.to_string() };
        self.subs.push(sub);
        self.renumber();
        println!("Субтитр добавлен.");
    }
    fn edit(&mut self, idx: usize, start_str: Option<&str>, end_str: Option<&str>, text: Option<&str>) {
        if idx < 1 || idx > self.subs.len() {
            println!("Неверный номер.");
            return;
        }
        let sub = &mut self.subs[idx-1];
        if let Some(s) = start_str {
            if let Ok(st) = parse_time(s) { sub.start = st; }
        }
        if let Some(e) = end_str {
            if let Ok(en) = parse_time(e) { sub.end = en; }
        }
        if let Some(t) = text {
            sub.text = t.to_string();
        }
        println!("Субтитр обновлён.");
    }
    fn delete(&mut self, idx: usize) {
        if idx < 1 || idx > self.subs.len() {
            println!("Неверный номер.");
            return;
        }
        self.subs.remove(idx-1);
        self.renumber();
        println!("Субтитр удалён.");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Использование: cargo run -- [--shift мс] [--check] [--output file] файл.srt");
        return;
    }
    let mut filename = String::new();
    let mut shift_ms = 0;
    let mut check_only = false;
    let mut output_file = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--shift" => {
                if i+1 < args.len() {
                    shift_ms = args[i+1].parse().unwrap_or(0);
                    i += 2;
                } else { i += 1; }
            }
            "--check" => { check_only = true; i += 1; }
            "--output" => {
                if i+1 < args.len() {
                    output_file = Some(args[i+1].clone());
                    i += 2;
                } else { i += 1; }
            }
            _ => { filename = args[i].clone(); i += 1; }
        }
    }
    if filename.is_empty() {
        println!("Не указан файл.");
        return;
    }
    let mut editor = match Editor::load(&filename) {
        Ok(e) => e,
        Err(e) => { println!("Ошибка загрузки: {}", e); return; }
    };
    if check_only {
        let errs = editor.validate();
        if errs.is_empty() {
            println!("Субтитры корректны.");
        } else {
            println!("Ошибки:");
            for e in errs { println!("  {}", e); }
        }
        return;
    }
    if shift_ms != 0 {
        editor.shift(shift_ms);
        if let Some(out) = output_file {
            editor.save(Some(&out)).unwrap();
        } else {
            editor.save(None).unwrap();
        }
        return;
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    println!("Редактор субтитров. Введите help для списка команд.");
    loop {
        print!("> ");
        stdout.flush().unwrap();
        let mut line = String::new();
        stdin.read_line(&mut line).unwrap();
        let cmd = line.trim();
        if cmd.is_empty() { continue; }
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let op = parts[0];
        match op {
            "quit" => break,
            "help" => println!("list, show <номер>, add, edit <номер> [время_начала] [время_конца] [текст], delete <номер>, shift <мс>, renumber, validate, save, quit"),
            "list" => editor.list(),
            "show" => {
                if parts.len() < 2 { println!("Укажите номер."); continue; }
                if let Ok(idx) = parts[1].parse::<usize>() {
                    if idx >= 1 && idx <= editor.subs.len() {
                        let sub = &editor.subs[idx-1];
                        println!("#{}  {} --> {}\n{}", sub.index, format_time(sub.start), format_time(sub.end), sub.text);
                    } else { println!("Неверный номер."); }
                }
            }
            "add" => {
                print!("Начало (HH:MM:SS,mmm): "); stdout.flush().unwrap();
                let mut start = String::new(); stdin.read_line(&mut start).unwrap();
                print!("Конец (HH:MM:SS,mmm): "); stdout.flush().unwrap();
                let mut end = String::new(); stdin.read_line(&mut end).unwrap();
                println!("Текст (введите END для завершения):");
                let mut text_lines = Vec::new();
                loop {
                    let mut line = String::new();
                    stdin.read_line(&mut line).unwrap();
                    if line.trim() == "END" { break; }
                    text_lines.push(line.trim().to_string());
                }
                let text = text_lines.join("\n");
                editor.add(&start.trim(), &end.trim(), &text);
            }
            "edit" => {
                if parts.len() < 2 { println!("Укажите номер."); continue; }
                let idx: usize = parts[1].parse().unwrap_or(0);
                print!("Новое начало (оставьте пустым): "); stdout.flush().unwrap();
                let mut start = String::new(); stdin.read_line(&mut start).unwrap();
                print!("Новый конец (оставьте пустым): "); stdout.flush().unwrap();
                let mut end = String::new(); stdin.read_line(&mut end).unwrap();
                print!("Новый текст (оставьте пустым): "); stdout.flush().unwrap();
                let mut text = String::new(); stdin.read_line(&mut text).unwrap();
                editor.edit(idx, if start.trim().is_empty() { None } else { Some(start.trim()) },
                            if end.trim().is_empty() { None } else { Some(end.trim()) },
                            if text.trim().is_empty() { None } else { Some(text.trim()) });
            }
            "delete" => {
                if parts.len() < 2 { println!("Укажите номер."); continue; }
                let idx: usize = parts[1].parse().unwrap_or(0);
                editor.delete(idx);
            }
            "shift" => {
                if parts.len() < 2 { println!("Укажите сдвиг в мс."); continue; }
                let ms: i64 = parts[1].parse().unwrap_or(0);
                editor.shift(ms);
            }
            "renumber" => editor.renumber(),
            "validate" => {
                let errs = editor.validate();
                if errs.is_empty() { println!("OK"); } else {
                    println!("Ошибки:");
                    for e in errs { println!("  {}", e); }
                }
            }
            "save" => {
                let fname = if parts.len() > 1 { Some(parts[1]) } else { None };
                if let Err(e) = editor.save(fname) {
                    println!("Ошибка сохранения: {}", e);
                } else {
                    println!("Сохранено.");
                }
            }
            _ => println!("Неизвестная команда."),
        }
    }
}
