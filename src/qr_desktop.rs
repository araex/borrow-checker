use screenshots::Screen;

/// Scan a selected area of the screen for QR codes
#[tauri::command]
pub async fn scan_screen_area(
    window: tauri::WebviewWindow,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<String, String> {
    log::info!("Scanning screen area: x={}, y={}, w={}, h={}", x, y, width, height);
    
    let screens = Screen::all()
        .map_err(|e| format!("Failed to get screens: {}", e))?;
    
    let screen = screens.first()
        .ok_or("No screen found")?;
    
    let image = screen.capture_area(x, y, width, height)
        .map_err(|e| format!("Failed to capture area: {}", e))?;
    
    let decoded = decode_qrcode(&image)?;
    
    // Close the window after successful scan
    if !decoded.is_empty() {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));
            let _ = window.close();
        });
    }
    
    Ok(decoded)
}

fn decode_qrcode(image: &screenshots::image::RgbaImage) -> Result<String, String> {
    let decoder = bardecoder::default_decoder();
    let results: Vec<String> = decoder.decode(image)
        .into_iter()
        .flatten()
        .collect();
    
    if results.is_empty() {
        Err("No QR code found in the selected area".to_string())
    } else {
        Ok(results[0].clone())
    }
}
