// src/app.rs
use ratatui::{layout::Rect, widgets::ListState};
use regex::Regex;
use std::{
    collections::{BTreeMap, VecDeque},
    process::Command,
    time::Instant,
};
use sysinfo::{Networks, System};

pub const SPARKLINE_LEN: usize = 100;

#[derive(PartialEq)]
pub enum AppMode {
    Normal,
    NicMenu,
    About,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Focus {
    NicBtn,
    ToggleBtn,
    FilterBtn,
    AboutBtn,
    QuitBtn,
}

pub struct NicInfo {
    pub hardware_model: String,
    pub driver: String,
    pub firmware: String,
    pub bus_info: String,
    pub xdp_capability: String,
    pub xdp_is_zerocopy: bool,
    pub current_xdp_state: String,
    pub mac_address: String,
    
    pub current_channels: usize,
    pub max_channels: usize,
    pub fallback_queues: usize,
}

pub struct App {
    pub networks: Networks,
    pub nics: Vec<(String, bool)>, 
    
    pub mode: AppMode,
    pub focus: Option<Focus>,
    pub selected_idx: usize,
    pub menu_state: ListState,
    pub show_throughput: bool,
    pub filter_drv_only: bool,

    pub physical_cores: usize,

    pub current_nic_info: Option<NicInfo>,

    pub mouse_pos: (u16, u16),
    pub btn_nic_rect: Rect,
    pub btn_toggle_rect: Rect,
    pub btn_filter_rect: Rect,
    pub btn_about_rect: Rect,
    pub btn_quit_rect: Rect,
    
    pub list_rect: Rect,
    pub hovered_nic_idx: Option<usize>,

    pub rx_bytes_history: VecDeque<u64>,
    pub tx_bytes_history: VecDeque<u64>,
    
    pub last_rx_bytes: u64,
    pub last_tx_bytes: u64,
    pub last_rx_packets: u64,
    pub last_tx_packets: u64,
    
    pub current_rx_bps: f64,
    pub current_tx_bps: f64,
    pub current_rx_pps: f64,
    pub current_tx_pps: f64,

    pub last_xdp_redirect_packets: u64,
    pub current_xdp_redirect_pps: f64,

    pub rx_queue_packets: BTreeMap<usize, u64>,
    pub last_rx_queue_packets: BTreeMap<usize, u64>,
    pub rx_queue_pps: BTreeMap<usize, f64>,

    pub rx_queue_xdp_packets: BTreeMap<usize, u64>,
    pub last_rx_queue_xdp_packets: BTreeMap<usize, u64>,
    pub rx_queue_xdp_pps: BTreeMap<usize, f64>,

    pub tx_queue_packets: BTreeMap<usize, u64>,
    pub last_tx_queue_packets: BTreeMap<usize, u64>,
    pub tx_queue_pps: BTreeMap<usize, f64>,

    pub last_update: Instant,
    pub quit: bool,
}

impl App {
    pub fn new_empty() -> Self {
        Self {
            networks: Networks::new(),
            nics: vec![],
            mode: AppMode::Normal,
            focus: None,
            selected_idx: 0,
            menu_state: ListState::default(),
            show_throughput: true,
            filter_drv_only: false,
            physical_cores: 4,
            current_nic_info: None,
            mouse_pos: (0, 0),
            btn_nic_rect: Rect::default(),
            btn_toggle_rect: Rect::default(),
            btn_filter_rect: Rect::default(),
            btn_about_rect: Rect::default(),
            btn_quit_rect: Rect::default(),
            list_rect: Rect::default(),
            hovered_nic_idx: None,
            rx_bytes_history: vec![0; SPARKLINE_LEN].into_iter().collect(),
            tx_bytes_history: vec![0; SPARKLINE_LEN].into_iter().collect(),
            last_rx_bytes: 0,
            last_tx_bytes: 0,
            last_rx_packets: 0,
            last_tx_packets: 0,
            current_rx_bps: 0.0,
            current_tx_bps: 0.0,
            current_rx_pps: 0.0,
            current_tx_pps: 0.0,
            last_xdp_redirect_packets: 0,
            current_xdp_redirect_pps: 0.0,
            rx_queue_packets: BTreeMap::new(),
            last_rx_queue_packets: BTreeMap::new(),
            rx_queue_pps: BTreeMap::new(),
            rx_queue_xdp_packets: BTreeMap::new(),
            last_rx_queue_xdp_packets: BTreeMap::new(),
            rx_queue_xdp_pps: BTreeMap::new(),
            tx_queue_packets: BTreeMap::new(),
            last_tx_queue_packets: BTreeMap::new(),
            tx_queue_pps: BTreeMap::new(),
            last_update: Instant::now(),
            quit: false,
        }
    }

    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let networks = Networks::new_with_refreshed_list();
        
        let mut temp_nics: Vec<(String, bool, u64)> = Vec::new();
        
        for (name, net_data) in networks.iter() {
            let device_path = format!("/sys/class/net/{}/device", name);
            if std::fs::metadata(&device_path).is_ok() {
                let mut is_zc = false;
                if let Ok(output) = Command::new("ethtool").arg("-i").arg(name).output() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if line.starts_with("driver:") { 
                            let drv = line.replace("driver:", "").trim().to_string();
                            is_zc = ["mlx5_core", "mlx4_core", "i40e", "ixgbe", "ice", "igb", "igc", "bnxt_en", "ena", "sfc", "virtio_net", "ntb"].contains(&drv.as_str());
                        }
                    }
                }
                let traffic = net_data.total_received() + net_data.total_transmitted();
                temp_nics.push((name.to_string(), is_zc, traffic));
            }
        }
        
        temp_nics.sort_by(|a, b| a.0.cmp(&b.0));
        if temp_nics.is_empty() {
            temp_nics.push(("lo".to_string(), false, 0));
        }

        let mut best_idx = 0;
        let mut best_score: u64 = 0;
        
        for (i, (name, is_zc, traffic)) in temp_nics.iter().enumerate() {
            let mut score = *traffic;
            if *is_zc { score += 100_000_000_000; }
            
            if let Ok(out) = Command::new("ip").arg("-details").arg("link").arg("show").arg("dev").arg(name).output() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.contains("xdpdrv") || stdout.contains("xdpoffload") {
                    score += 500_000_000_000;
                } else if stdout.contains("xdpgeneric") {
                    score += 200_000_000_000;
                }
            }
            if score >= best_score {
                best_score = score;
                best_idx = i;
            }
        }

        let nics = temp_nics.into_iter().map(|(n, z, _)| (n, z)).collect();
        let physical_cores = sys.physical_core_count().unwrap_or_else(|| sys.cpus().len());

        let mut menu_state = ListState::default();
        menu_state.select(Some(best_idx));

        let mut app = Self::new_empty();
        app.networks = networks;
        app.nics = nics;
        app.selected_idx = best_idx;
        app.menu_state = menu_state;
        app.physical_cores = physical_cores;

        app.fetch_nic_info();
        app
    }

    pub fn select_nic(&mut self, idx: usize) {
        self.selected_idx = idx;
        self.reset_stats();
        self.fetch_nic_info();
    }

    pub fn next_nic(&mut self) {
        let next = (self.selected_idx + 1) % self.nics.len();
        self.select_nic(next);
    }

    pub fn prev_nic(&mut self) {
        let prev = if self.selected_idx == 0 {
            self.nics.len() - 1
        } else {
            self.selected_idx - 1
        };
        self.select_nic(prev);
    }

    pub fn reset_stats(&mut self) {
        self.rx_bytes_history = vec![0; SPARKLINE_LEN].into_iter().collect();
        self.tx_bytes_history = vec![0; SPARKLINE_LEN].into_iter().collect();
        self.last_rx_bytes = 0;
        self.last_tx_bytes = 0;
        self.last_rx_packets = 0;
        self.last_tx_packets = 0;
        
        self.last_xdp_redirect_packets = 0;
        self.current_xdp_redirect_pps = 0.0;

        self.rx_queue_packets.clear();
        self.last_rx_queue_packets.clear();
        self.rx_queue_pps.clear();

        self.rx_queue_xdp_packets.clear();
        self.last_rx_queue_xdp_packets.clear();
        self.rx_queue_xdp_pps.clear();

        self.tx_queue_packets.clear();
        self.last_tx_queue_packets.clear();
        self.tx_queue_pps.clear();

        self.last_update = Instant::now();
    }

    pub fn fetch_nic_info(&mut self) {
        let nic_name = &self.nics[self.selected_idx].0;
        
        let mut driver = String::from("Unknown");
        let mut firmware = String::from("Unknown");
        let mut bus_info = String::from("Unknown");
        
        if let Ok(output) = Command::new("ethtool").arg("-i").arg(nic_name).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("driver:") { driver = line.replace("driver:", "").trim().to_string(); }
                if line.starts_with("firmware-version:") { firmware = line.replace("firmware-version:", "").trim().to_string(); }
                if line.starts_with("bus-info:") { bus_info = line.replace("bus-info:", "").trim().to_string(); }
            }
        }

        let mut hardware_model = String::from("Unknown (Not PCI/Virtual)");
        if !bus_info.is_empty() && bus_info != "Unknown" {
            if let Ok(output) = Command::new("lspci").arg("-s").arg(&bus_info).output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().next() {
                    if let Some(idx) = line.find(": ") {
                        hardware_model = line[idx+2..].trim().to_string();
                    }
                }
            }
        }

        let xdp_is_zerocopy = self.nics[self.selected_idx].1;
        let xdp_capability = if xdp_is_zerocopy {
            "Supported (Native 'drv' Zero-Copy Capable)".to_string()
        } else {
            "Restricted (SKB Software Mode Only)".to_string()
        };

        let mut current_xdp_state = String::from("None (Standard Linux Networking)");
        let mut mac_address = String::from("Unknown");
        if let Ok(output) = Command::new("ip").arg("-details").arg("link").arg("show").arg("dev").arg(nic_name).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if stdout.contains("xdpdrv") || (stdout.contains("xdp ") && !stdout.contains("xdpgeneric")) {
                current_xdp_state = "Active - NATIVE (drv)".to_string();
            } else if stdout.contains("xdpgeneric") {
                current_xdp_state = "Active - GENERIC (skb)".to_string();
            } else if stdout.contains("xdpoffload") {
                current_xdp_state = "Active - OFFLOAD (hw)".to_string();
            } else if stdout.contains("xdp") {
                current_xdp_state = "Active - Unknown Mode".to_string();
            }
            
            if let Some(mac_line) = stdout.lines().find(|l| l.contains("link/ether")) {
                let parts: Vec<&str> = mac_line.split_whitespace().collect();
                if parts.len() >= 2 {
                    mac_address = parts[1].to_string();
                }
            }
        }

        let mut max_channels = 0;
        let mut current_channels = 0;
        
        if let Ok(output) = Command::new("ethtool").arg("-l").arg(nic_name).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut in_max = false;
            let mut in_current = false;
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Pre-set maximums:") {
                    in_max = true;
                    in_current = false;
                } else if trimmed.starts_with("Current hardware settings:") {
                    in_max = false;
                    in_current = true;
                } else if trimmed.starts_with("Combined:") || trimmed.starts_with("RX:") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() == 2 {
                        if let Ok(v) = parts[1].parse::<usize>() {
                            if v > 0 {
                                if in_max { max_channels = std::cmp::max(max_channels, v); }
                                if in_current { current_channels = std::cmp::max(current_channels, v); }
                            }
                        }
                    }
                }
            }
        }

        let mut fallback_queues = 0;
        let queue_dir = format!("/sys/class/net/{}/queues", nic_name);
        if let Ok(entries) = std::fs::read_dir(queue_dir) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with("rx-") {
                    fallback_queues += 1;
                }
            }
        }
        if fallback_queues == 0 { fallback_queues = 1; }

        self.current_nic_info = Some(NicInfo {
            hardware_model,
            driver,
            firmware,
            bus_info,
            xdp_capability,
            xdp_is_zerocopy,
            current_xdp_state,
            mac_address,
            current_channels,
            max_channels,
            fallback_queues,
        });
    }

    pub fn parse_ethtool_output(&mut self, stdout: &str, elapsed: f64) {
        let rx_re = Regex::new(r"(?i)(?:^|\s)(?:port\.|vport_|rx_)?(?:rx|q|queue)[^\d]*(\d+)[^\s]*?(?:packets|pkts|cnt):\s+(\d+)").unwrap();
        let tx_re = Regex::new(r"(?i)(?:^|\s)(?:port\.|vport_|tx_)?(?:tx|q|queue)[^\d]*(\d+)[^\s]*?(?:packets|pkts|cnt):\s+(\d+)").unwrap();
        let xdp_redirect_re = Regex::new(r"(?i)(?:^|\s).*xdp(?:_redirect|_tx|_drop|_packets|_pkts)?.*:\s+(\d+)").unwrap();
        let rx_xdp_re = Regex::new(r"(?i)(?:^|\s)(?:port\.|vport_|rx_)?(?:rx|q|queue)[^\d]*(\d+)[^\s]*xdp[^\s]*:\s+(\d+)").unwrap();
        
        let mut total_xdp_redirect: u64 = 0;

        self.rx_queue_xdp_pps.clear();
        self.rx_queue_xdp_packets.clear();
        
        self.rx_queue_pps.clear();
        self.rx_queue_packets.clear();

        for cap in rx_re.captures_iter(stdout) {
            let match_str = cap[0].to_lowercase();
            if match_str.contains("xdp") {
                continue;
            }

            if let (Ok(q_id), Ok(pkts)) = (cap[1].parse::<usize>(), cap[2].parse::<u64>()) {
                let last_pkts = self.last_rx_queue_packets.get(&q_id).copied().unwrap_or(pkts);
                let diff = pkts.saturating_sub(last_pkts);
                let pps = diff as f64 / elapsed;
                
                self.rx_queue_pps.insert(q_id, pps);
                self.rx_queue_packets.insert(q_id, pkts);
            }
        }

        for cap in rx_xdp_re.captures_iter(stdout) {
            if let (Ok(q_id), Ok(pkts)) = (cap[1].parse::<usize>(), cap[2].parse::<u64>()) {
                let current = self.rx_queue_xdp_packets.entry(q_id).or_insert(0);
                *current += pkts;
            }
        }

        for (&q_id, &pkts) in &self.rx_queue_xdp_packets {
            let last_pkts = self.last_rx_queue_xdp_packets.get(&q_id).copied().unwrap_or(pkts);
            let diff = pkts.saturating_sub(last_pkts);
            let pps = diff as f64 / elapsed;
            self.rx_queue_xdp_pps.insert(q_id, pps);
        }
        self.last_rx_queue_xdp_packets = self.rx_queue_xdp_packets.clone();

        self.tx_queue_pps.clear();
        self.tx_queue_packets.clear();

        for cap in tx_re.captures_iter(stdout) {
            let match_str = cap[0].to_lowercase();
            if match_str.contains("xdp") || match_str.contains("rx") {
                continue;
            }

            if let (Ok(q_id), Ok(pkts)) = (cap[1].parse::<usize>(), cap[2].parse::<u64>()) {
                let last_pkts = self.last_tx_queue_packets.get(&q_id).copied().unwrap_or(pkts);
                let diff = pkts.saturating_sub(last_pkts);
                let pps = diff as f64 / elapsed;
                
                self.tx_queue_pps.insert(q_id, pps);
                self.tx_queue_packets.insert(q_id, pkts);
            }
        }

        for cap in xdp_redirect_re.captures_iter(stdout) {
            if let Ok(val) = cap[1].parse::<u64>() {
                total_xdp_redirect += val;
            }
        }

        if self.last_xdp_redirect_packets > 0 {
            let xdp_diff = total_xdp_redirect.saturating_sub(self.last_xdp_redirect_packets);
            self.current_xdp_redirect_pps = xdp_diff as f64 / elapsed;
        }
        self.last_xdp_redirect_packets = total_xdp_redirect;

        self.last_rx_queue_packets = self.rx_queue_packets.clone();
        self.last_tx_queue_packets = self.tx_queue_packets.clone();
    }

    pub fn update_stats(&mut self) {
        let nic_name = self.nics[self.selected_idx].0.clone();
        self.networks.refresh();
        
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        
        // Prevents division by extremely small numbers causing infinite PPS, leading to UI distortion
        if elapsed < 0.1 { return; } 

        if let Some(network) = self.networks.get(&nic_name) {
            let rx_bytes = network.total_received();
            let tx_bytes = network.total_transmitted();
            let rx_packets = network.total_packets_received();
            let tx_packets = network.total_packets_transmitted();

            if self.last_rx_bytes > 0 {
                self.current_rx_bps = ((rx_bytes.saturating_sub(self.last_rx_bytes)) as f64 * 8.0) / elapsed;
                self.current_tx_bps = ((tx_bytes.saturating_sub(self.last_tx_bytes)) as f64 * 8.0) / elapsed;
                self.current_rx_pps = (rx_packets.saturating_sub(self.last_rx_packets)) as f64 / elapsed;
                self.current_tx_pps = (tx_packets.saturating_sub(self.last_tx_packets)) as f64 / elapsed;

                self.rx_bytes_history.pop_front();
                self.rx_bytes_history.push_back(self.current_rx_bps as u64);
                self.tx_bytes_history.pop_front();
                self.tx_bytes_history.push_back(self.current_tx_bps as u64);
            }

            self.last_rx_bytes = rx_bytes;
            self.last_tx_bytes = tx_bytes;
            self.last_rx_packets = rx_packets;
            self.last_tx_packets = tx_packets;
        }

        if let Ok(output) = Command::new("ethtool").arg("-S").arg(&nic_name).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.parse_ethtool_output(&stdout, elapsed);
        }

        self.last_update = now;
    }
}