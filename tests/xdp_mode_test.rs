// tests/xdp_mode_test.rs

/*
 * * 1. SKB Mode (Generic XDP):
 * In SKB mode, the XDP program is executed by the core kernel network stack *after* * the hardware driver has already allocated the socket buffer (SKB) and passed it up. 
 * Because the hardware driver is entirely unaware of this execution, it does NOT 
 * increment any hardware-level XDP counters (like `xdp_redirect` or `xdp_drop`).
 * Therefore, `ethtool -S` will only show standard `rx_packets` incrementing.
 * * 2. DRV Mode (Native XDP / Zero-Copy):
 * In DRV mode, the XDP program executes directly inside the hardware driver's NAPI 
 * poll loop, *before* memory is allocated for an SKB. When the XDP program issues 
 * an `XDP_REDIRECT`, `XDP_DROP`, or `XDP_TX`, the driver registers this action and 
 * increments its specific hardware counters. These are exposed via `ethtool -S`.
 * * By parsing `ethtool -S`, we are reading directly from the hardware driver. If XDP 
 * counters are incrementing, we are 100% certain the packets are being handled in 
 * DRV/Native mode.
 * * Caveat (XDP_PASS in Native Mode):
 * If an XDP program is attached in Native mode but returns `XDP_PASS` for 100% of 
 * packets, some drivers do not maintain an `rx_xdp_pass` counter. They simply count 
 * them as standard `rx_packets`. In this scenario, the queue throughput will appear 
 * as SKB (Green). This is functionally accurate since the packets are being passed to 
 * the SKB software stack anyway. The UI handles this edge case by reading the `ip link` 
 * state and displaying:
 * "XDP Native Active (No Zero-Copy / Redirect stats detected. Packets passed to SKB...)"
 * =========================================================================================
 */

use xdp_top::app::App;

#[test]
fn test_drv_vs_skb_mode_detection() {
    let mut app = App::new_empty();

    // 0. Establish Baseline
    // The app avoids massive PPS spikes on the very first read by setting 
    // the baseline to the current counter value. We must simulate this initial fetch.
    let baseline_output = "
        rx_queue_0_packets: 0
        rx_queue_0_xdp_packets: 0
        rx_queue_1_packets: 0
    ";
    app.parse_ethtool_output(baseline_output, 1.0);

    // 1. Simulate standard SKB traffic (Normal Networking)
    // Here we provide ethtool -S output where rx queues receive packets 
    // but there are no corresponding XDP redirection packets for them.
    let skb_output = "
        rx_queue_0_packets: 1000
        rx_queue_0_xdp_packets: 0
        rx_queue_1_packets: 50
    ";
    
    // Elapsed time of 1.0 second since baseline
    app.parse_ethtool_output(skb_output, 1.0);
    
    assert_eq!(app.rx_queue_pps.get(&0).copied().unwrap_or(0.0), 1000.0);
    assert_eq!(app.rx_queue_xdp_pps.get(&0).copied().unwrap_or(0.0), 0.0);
    
    let pps = *app.rx_queue_pps.get(&0).unwrap();
    let xdp_pps = *app.rx_queue_xdp_pps.get(&0).unwrap_or(&0.0);
    
    // UI Logic Validation for SKB mode detection
    let is_zc = xdp_pps > (pps * 0.1) || xdp_pps > 10.0;
    assert!(!is_zc, "SKB traffic incorrectly identified as DRV/Zero-Copy");

    // 2. Simulate DRV (Native XDP / Zero-Copy) traffic
    // Now provide simulated data that XDP drops/redirects packets at the driver queue level.
    // 1 second elapses from the last fetch. The counters increment cumulatively.
    let drv_output = "
        rx_queue_0_packets: 5000
        rx_queue_0_xdp_packets: 4000
        rx_queue_1_packets: 50
    ";
    
    app.parse_ethtool_output(drv_output, 1.0);
    
    // Validation for PPS diff calculations
    // rx_queue_0: 5000 (current) - 1000 (last) = 4000 pps total flow
    assert_eq!(app.rx_queue_pps.get(&0).copied().unwrap_or(0.0), 4000.0);
    
    // rx_queue_0_xdp: 4000 (current) - 0 (last) = 4000 pps handled strictly by XDP
    assert_eq!(app.rx_queue_xdp_pps.get(&0).copied().unwrap_or(0.0), 4000.0);

    let pps_drv = *app.rx_queue_pps.get(&0).unwrap();
    let xdp_pps_drv = *app.rx_queue_xdp_pps.get(&0).unwrap_or(&0.0);
    
    // UI Logic Validation for DRV mode detection
    let is_zc_drv = xdp_pps_drv > (pps_drv * 0.1) || xdp_pps_drv > 10.0;
    assert!(is_zc_drv, "DRV traffic failed to be correctly identified as Zero-Copy/XDP");
}