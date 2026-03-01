use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, Block, Borders, Clear, List, ListItem, Paragraph, Sparkline},
    Frame,
};

use crate::app::{App, AppMode, Focus};

pub fn is_inside(pos: (u16, u16), rect: Rect) -> bool {
    pos.0 >= rect.x && pos.0 < rect.x + rect.width &&
    pos.1 >= rect.y && pos.1 < rect.y + rect.height
}

pub fn format_bps(bits: f64) -> String {
    const K: f64 = 1_000.0;
    const M: f64 = K * 1_000.0;
    const G: f64 = M * 1_000.0;
    const T: f64 = G * 1_000.0;
    
    if bits >= T { format!("{:.2} Tbps", bits / T) }
    else if bits >= G { format!("{:.2} Gbps", bits / G) }
    else if bits >= M { format!("{:.2} Mbps", bits / M) }
    else if bits >= K { format!("{:.2} Kbps", bits / K) }
    else { format!("{:.0} bps", bits) }
}

pub fn format_pps(pkts: f64) -> String {
    const K: f64 = 1_000.0;
    const M: f64 = K * 1_000.0;
    if pkts >= M { format!("{:.2} Mpps", pkts / M) }
    else if pkts >= K { format!("{:.2} Kpps", pkts / K) }
    else { format!("{:.0} p/s", pkts) }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn draw_ui(f: &mut Frame, app: &mut App) {
    // Dynamically size the layout based on the throughput toggle
    let mut root_constraints = vec![
        Constraint::Length(3),  // 0: Header Row
        Constraint::Length(7),  // 1: Split NIC Info pane
        Constraint::Length(4),  // 2: Efficiency
    ];

    if app.show_throughput {
        root_constraints.push(Constraint::Length(10)); // 3: Sparklines
    }
    
    root_constraints.push(Constraint::Fill(1)); // RX BarChart
    root_constraints.push(Constraint::Fill(1)); // TX BarChart

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(root_constraints)
        .split(f.size());

    // 1. Header Row & Buttons
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(25),      // Info block
            Constraint::Length(18),   // Select NIC
            Constraint::Length(23),   // Toggle Throughput
            Constraint::Length(11),   // About
            Constraint::Length(10),   // Quit
        ])
        .split(chunks[0]);

    app.btn_nic_rect = header_chunks[1];
    app.btn_toggle_rect = header_chunks[2];
    app.btn_about_rect = header_chunks[3];
    app.btn_quit_rect = header_chunks[4];

    let header_text = Line::from(vec![
        Span::styled(" XDP-TOP ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled(format!("< {} >", app.nics[app.selected_idx].0), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(format!(" | CPU: {} Cores", app.physical_cores)),
    ]);
    let title_block = Paragraph::new(header_text).block(Block::default().borders(Borders::ALL));
    f.render_widget(title_block, header_chunks[0]);

    // Unified button styling
    let idle_btn_style = Style::default().fg(Color::Cyan);
    let focus_btn_style = Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD);

    let nic_focused = app.focus == Some(Focus::NicBtn) || is_inside(app.mouse_pos, app.btn_nic_rect);
    let nic_btn = Paragraph::new(" [ Select NIC ] ").alignment(Alignment::Center).block(Block::default().borders(Borders::ALL).style(if nic_focused { focus_btn_style } else { idle_btn_style }));
    f.render_widget(nic_btn, header_chunks[1]);

    let toggle_txt = if app.show_throughput { " [ Hide Throughput ] " } else { " [ Show Throughput ] " };
    let toggle_focused = app.focus == Some(Focus::ToggleBtn) || is_inside(app.mouse_pos, app.btn_toggle_rect);
    let toggle_btn = Paragraph::new(toggle_txt).alignment(Alignment::Center).block(Block::default().borders(Borders::ALL).style(if toggle_focused { focus_btn_style } else { idle_btn_style }));
    f.render_widget(toggle_btn, header_chunks[2]);

    let about_focused = app.focus == Some(Focus::AboutBtn) || is_inside(app.mouse_pos, app.btn_about_rect);
    let about_btn = Paragraph::new(" [ About ] ").alignment(Alignment::Center).block(Block::default().borders(Borders::ALL).style(if about_focused { focus_btn_style } else { idle_btn_style }));
    f.render_widget(about_btn, header_chunks[3]);

    let quit_focused = app.focus == Some(Focus::QuitBtn) || is_inside(app.mouse_pos, app.btn_quit_rect);
    let quit_btn = Paragraph::new(" [ Quit ] ").alignment(Alignment::Center).block(Block::default().borders(Borders::ALL).style(if quit_focused { focus_btn_style } else { idle_btn_style }));
    f.render_widget(quit_btn, header_chunks[4]);

    // 2. NIC Info (Split Horizontally)
    let info_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);

    if let Some(info) = &app.current_nic_info {
        // Hardware Box
        let hw_text = vec![
            Line::from(vec![Span::styled("Hardware:  ", Style::default().fg(Color::DarkGray)), Span::styled(&info.hardware_model, Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled("Driver:    ", Style::default().fg(Color::DarkGray)), Span::styled(&info.driver, Style::default().fg(Color::LightBlue)), Span::raw(format!(" (fw: {})", info.firmware))]),
            Line::from(vec![Span::styled("Bus/MAC:   ", Style::default().fg(Color::DarkGray)), Span::raw(format!("{} | {}", info.bus_info, info.mac_address))]),
        ];
        let hw_block = Paragraph::new(hw_text).block(Block::default().borders(Borders::ALL).title(" Hardware Details "));
        f.render_widget(hw_block, info_chunks[0]);

        // Channel / XDP Box
        let cap_color = if info.xdp_is_zerocopy { Color::Green } else { Color::Yellow };
        let state_color = if info.current_xdp_state.contains("NATIVE") { Color::Green }
                          else if info.current_xdp_state.contains("GENERIC") { Color::Red }
                          else { Color::DarkGray };

        let channels_text = if info.current_channels > 0 {
            format!("{} active channels (Max Supported: {})", info.current_channels, info.max_channels)
        } else {
            format!("{} active queues (Fallback detection)", info.fallback_queues)
        };

        let xdp_text = vec![
            Line::from(vec![Span::styled("Channels:  ", Style::default().fg(Color::DarkGray)), Span::raw(channels_text)]),
            Line::from(vec![Span::styled("XDP Cap:   ", Style::default().fg(Color::DarkGray)), Span::styled(&info.xdp_capability, Style::default().fg(cap_color).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled("XDP State: ", Style::default().fg(Color::DarkGray)), Span::styled(&info.current_xdp_state, Style::default().fg(state_color).add_modifier(Modifier::BOLD))]),
        ];
        let xdp_block = Paragraph::new(xdp_text).block(Block::default().borders(Borders::ALL).title(" Logical Channels & XDP "));
        f.render_widget(xdp_block, info_chunks[1]);

        // 3. Efficiency Assessment (Flow-Aware)
        let total_pps = app.current_rx_pps + app.current_tx_pps;
        let c_count = app.physical_cores;
        
        let (eff_color, eff_text) = if total_pps < 100.0 {
            (Color::DarkGray, "Awaiting sufficient traffic flow (>100 pps) to accurately analyze RSS hashing efficiency...".to_string())
        } else {
            let active_queues = app.rx_queue_pps.values().filter(|&&pps| pps > 5.0).count();
            if active_queues == 0 {
                (Color::Yellow, format!("Traffic flowing ({} pps), but driver is not reporting per-queue stats via ethtool.", format_pps(total_pps)))
            } else if active_queues <= c_count {
                (Color::Green, format!("OPTIMAL: Traffic is perfectly distributed across {} active queues, fitting within {} physical cores.", active_queues, c_count))
            } else {
                (Color::Red, format!("SUBOPTIMAL: Traffic is scattered across {} active queues, exceeding {} physical cores. Expect context switching overhead.", active_queues, c_count))
            }
        };

        let eff_paragraph = Paragraph::new(Line::from(Span::styled(eff_text, Style::default().fg(eff_color).add_modifier(Modifier::BOLD))))
            .block(Block::default().borders(Borders::ALL).title(" Live Data Flow Efficiency Assessment "));
        f.render_widget(eff_paragraph, chunks[2]);
    }

    // Calculate dynamic indices based on throughput toggle
    let rx_chunk_idx = if app.show_throughput { 4 } else { 3 };
    let tx_chunk_idx = if app.show_throughput { 5 } else { 4 };

    // 4. Traffic Sparklines (Conditional)
    if app.show_throughput {
        let spark_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[3]);

        let rx_data: Vec<u64> = app.rx_bytes_history.iter().copied().collect();
        let rx_title = format!(" RX Throughput: {} | {} ", format_bps(app.current_rx_bps), format_pps(app.current_rx_pps));
        let rx_sparkline = Sparkline::default()
            .block(Block::default().title(rx_title).borders(Borders::ALL))
            .data(&rx_data)
            .style(Style::default().fg(Color::Green));
        f.render_widget(rx_sparkline, spark_chunks[0]);

        let tx_data: Vec<u64> = app.tx_bytes_history.iter().copied().collect();
        let tx_title = format!(" TX Throughput: {} | {} ", format_bps(app.current_tx_bps), format_pps(app.current_tx_pps));
        let tx_sparkline = Sparkline::default()
            .block(Block::default().title(tx_title).borders(Borders::ALL))
            .data(&tx_data)
            .style(Style::default().fg(Color::Blue));
        f.render_widget(tx_sparkline, spark_chunks[1]);
    }

    // 5. Dense Queue Distribution BarCharts
    let mut rx_bars = Vec::new();
    let mut rx_labels = Vec::new(); 
    let mut sorted_rx: Vec<_> = app.rx_queue_pps.iter().collect();
    sorted_rx.sort_by_key(|k| k.0);

    for (q_id, _) in &sorted_rx { rx_labels.push(q_id.to_string()); } // Removed 'Q' prefix
    for (i, (_, pps)) in sorted_rx.into_iter().enumerate() {
        rx_bars.push(Bar::default().label(rx_labels[i].as_str().into()).value(*pps as u64));
    }

    if rx_bars.is_empty() {
        let no_data = Paragraph::new("No per-queue statistics available for this driver.")
            .block(Block::default().borders(Borders::ALL).title(" RX Queue Load Distribution (PPS) "));
        f.render_widget(no_data, chunks[rx_chunk_idx]);
    } else {
        let rx_barchart = BarChart::default()
            .block(Block::default().title(" RX Queue Load Distribution (PPS) ").borders(Borders::ALL))
            .data(ratatui::widgets::BarGroup::default().bars(&rx_bars))
            .bar_width(2) // Condensed to fit 60+ channels
            .bar_gap(1)
            .bar_style(Style::default().fg(Color::Green))
            .value_style(Style::default().fg(Color::Black).bg(Color::Green));
        f.render_widget(rx_barchart, chunks[rx_chunk_idx]);
    }

    let mut tx_bars = Vec::new();
    let mut tx_labels = Vec::new(); 
    let mut sorted_tx: Vec<_> = app.tx_queue_pps.iter().collect();
    sorted_tx.sort_by_key(|k| k.0);

    for (q_id, _) in &sorted_tx { tx_labels.push(q_id.to_string()); } // Removed 'Q' prefix
    for (i, (_, pps)) in sorted_tx.into_iter().enumerate() {
        tx_bars.push(Bar::default().label(tx_labels[i].as_str().into()).value(*pps as u64));
    }

    if tx_bars.is_empty() {
        let no_data = Paragraph::new("No per-queue statistics available for this driver.")
            .block(Block::default().borders(Borders::ALL).title(" TX Queue Load Distribution (PPS) "));
        f.render_widget(no_data, chunks[tx_chunk_idx]);
    } else {
        let tx_barchart = BarChart::default()
            .block(Block::default().title(" TX Queue Load Distribution (PPS) ").borders(Borders::ALL))
            .data(ratatui::widgets::BarGroup::default().bars(&tx_bars))
            .bar_width(2) // Condensed
            .bar_gap(1)
            .bar_style(Style::default().fg(Color::Blue))
            .value_style(Style::default().fg(Color::Black).bg(Color::Blue));
        f.render_widget(tx_barchart, chunks[tx_chunk_idx]);
    }

    // 6. OVERLAYS (Popups)
    if app.mode == AppMode::NicMenu {
        let popup_area = centered_rect(40, 50, f.size());
        f.render_widget(Clear, popup_area); 
        app.list_rect = popup_area;

        let items: Vec<ListItem> = app.nics.iter().enumerate().map(|(i, (name, is_zc))| {
            let prefix = if i == app.selected_idx { "[*]" } else { "[ ]" };
            let suffix = if *is_zc { " [Zerocopy capable]" } else { "" };
            
            let mut style = Style::default();
            if app.hovered_nic_idx == Some(i) {
                style = style.fg(Color::Black).bg(Color::White); // Mouse hover color
            }
            
            ListItem::new(format!("{} {}{}", prefix, name, suffix)).style(style)
        }).collect();

        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(" Select Physical Interface ")
                .border_style(Style::default().fg(Color::Cyan))
                .style(Style::default().bg(Color::Rgb(30, 30, 40)))) 
            .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");

        if app.hovered_nic_idx.is_some() {
            f.render_widget(list, popup_area);
        } else {
            f.render_stateful_widget(list, popup_area, &mut app.menu_state);
        }
        
    } else if app.mode == AppMode::About {
        let popup_area = centered_rect(42, 44, f.size());
        f.render_widget(Clear, popup_area);
        
        let about_text = vec![
            Line::from(""),
            Line::from(Span::styled("XDP-TOP v0.2.0", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))),
            Line::from(""),
            Line::from("A throughput visualizer for eXpress Data Path (XDP)"),
            Line::from("capable network cards."),
            Line::from(""),
            Line::from("Created by Christian Wittenberg."),
            Line::from("License: MIT Open Source"),
            Line::from(""),
            Line::from(Span::styled("Press Esc or Enter to close.", Style::default().fg(Color::DarkGray))),
        ];
        
        let block = Paragraph::new(about_text)
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(" About ")
                .border_style(Style::default().fg(Color::Cyan))
                .style(Style::default().bg(Color::Rgb(30, 30, 40)))); 
        f.render_widget(block, popup_area);
    }
}