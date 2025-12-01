#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::thread;
use std::fs;
use std::path::PathBuf;
use tiny_http::{Header, Method, Response, Server};
use tauri::{
    Emitter,
    Manager,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
    generate_context,
};
// use tauri_plugin_shell::ShellExt;
use reqwest;

#[derive(Deserialize)]
struct IncomingData {
    lat: f64,
    lng: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MapViewRequest {
    lat: f64,
    lng: f64,
    map_type: String, // "google" hoặc "openstreetmap"
    point_id: String, 
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleResult {
    // luôn có (không phải Option)
    pub status: String,
    pub address: String,

    // các field chính (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poi_vn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poi_en: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poi_ex: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poi_st_sd: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub house_num: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buaname: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub st_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_com: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fax: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mail: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub brandname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gen_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classify: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dtrend: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub be_id: Option<String>,

    // thêm một vài trường tiện lợi
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plus_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
}
// Cấu hình API mới với custom URL
#[derive(Debug, Clone, Serialize, Deserialize)]


struct ApiConfig {
    custom_url: String,
    opacity: f64,                                                                                                                                                     
}

// State để lưu trữ window và dữ liệu mới nhất
struct AppState {
    window: Arc<Mutex<Option<tauri::WebviewWindow>>>,
    latest_data: Arc<Mutex<Option<ExampleResult>>>,
    pending_requests: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<ExampleResult>>>>,
    api_config: Arc<Mutex<ApiConfig>>,
    is_processing: Arc<Mutex<bool>>,
}

// Hàm lưu cấu hình vào file
fn save_config(config: &ApiConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;
    let config_dir = config_path.parent().unwrap();
    
    // Tạo thư mục nếu chưa tồn tại
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }
    
    let config_json = serde_json::to_string_pretty(config)?;
    fs::write(&config_path, config_json)?;
    println!("Configuration saved to: {:?}", config_path);
    Ok(())
}

// Hàm load cấu hình từ file
fn load_config() -> ApiConfig {
    match get_config_path() {
        Ok(config_path) => {
            if config_path.exists() {
                match fs::read_to_string(&config_path) {
                    Ok(content) => {
                        match serde_json::from_str::<ApiConfig>(&content) {
                            Ok(config) => {
                                println!("Configuration loaded from: {:?}", config_path);
                                return config;
                            }
                            Err(e) => {
                                println!("Error parsing config file: {}, using default", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error reading config file: {}, using default", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("Error getting config path: {}, using default", e);
        }
    }
    
    // Trả về config mặc định nếu không load được
    ApiConfig::default()
}

// Hàm lấy đường dẫn file config
fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = dirs::config_dir().ok_or("Cannot find config directory")?;
    path.push("GeocoderApp");
    path.push("config.json");
    Ok(path)
}

// Hàm gọi Custom API
async fn call_custom_api(lat: f64, lng: f64, custom_url: &str) -> Result<ExampleResult, Box<dyn std::error::Error>> {
    let url = custom_url
        .replace("{lat}", &lat.to_string())
        .replace("{lng}", &lng.to_string())
        .replace("{long}", &lng.to_string()); // Support both {lng} and {long}

    println!("Calling Custom API: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let response_text = response.text().await?;
    println!("Custom API response: {}", response_text);

    // Parse response từ Google Geocoding API
    let google_response: serde_json::Value = serde_json::from_str(&response_text)?;
    
    // Tạo ExampleResult từ dữ liệu Google Geocoding
    let result = parse_google_geocoding_response(google_response);
    
    Ok(result)
}

// Hàm parse dữ liệu từ Google Geocoding API
pub fn parse_google_geocoding_response(response: Value) -> ExampleResult {
    let mut result = ExampleResult {
        status: "success".to_string(),
        address: "".to_string(),

        poi_vn: None,
        poi_en: None,
        poi_ex: None,

        r#type: None,
        sub_type: None,
        poi_st_sd: None,

        room: None,
        house_num: None,
        buaname: None,

        st_name: None,
        sub_com: None,

        phone: None,
        fax: None,
        web: None,
        mail: None,

        brandname: None,
        import: None,
        status_detail: None,
        note: None,
        done: None,
        update_: None,
        source: None,
        gen_type: None,
        perform: None,
        dup: None,
        explain: None,
        classify: None,
        dtrend: None,

        google_id: None,
        be_id: None,

        plus_code: None,
        latitude: None,
        longitude: None,
    };

    // Lấy status
    if let Some(status) = response["status"].as_str() {
        if status != "OK" {
            result.status = status.to_string();
            return result;
        }
    }

    let first = &response["results"][0];

    result.address = first["formatted_address"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    if let Some(loc) = first["geometry"]["location"].as_object() {
        result.latitude = loc.get("lat").and_then(|v| v.as_f64());
        result.longitude = loc.get("lng").and_then(|v| v.as_f64());
    }

    result.plus_code = first["plus_code"]["global_code"]
        .as_str()
        .map(|s| s.to_string());

    if let Some(components) = first["address_components"].as_array() {
        for comp in components {
            let long = comp["long_name"].as_str().unwrap_or("").to_string();
            let types = comp["types"].as_array().map_or(&[] as &[serde_json::Value], |v| v);

            let is = |t: &str| types.iter().any(|x| x.as_str() == Some(t));

            // Map vào các trường yêu cầu
            if is("premise") || is("point_of_interest") {
                result.poi_vn = Some(long.clone());
            }
            if is("street_number") {
                result.house_num = Some(long.clone());
            }
            if is("floor") || is("room") {
                result.room = Some(long.clone());
            }
            if is("sublocality_level_1") {
                result.buaname = Some(long.clone());
            }
            if is("sublocality_level_2") {
                result.sub_com = Some(long.clone());
            }
            if is("route") {
                result.st_name = Some(long.clone());
            }
        }
    }

    result.phone = first["formatted_phone_number"].as_str().map(|s| s.to_string());
    result.web = first["website"].as_str().map(|s| s.to_string());
    result.google_id = first["place_id"].as_str().map(|s| s.to_string());

    result
}


fn start_local_server(app_state: Arc<AppState>) {
    thread::spawn(move || {
        let server = Server::http("127.0.0.1:31203").unwrap();
        println!("Tauri server listening on http://127.0.0.1:31203");

        loop {
            let mut request = match server.recv() {
                Ok(rq) => rq,
                Err(e) => {
                    println!("Server error: {}", e);
                    continue;
                }
            };

            if request.method() == &Method::Post && request.url() == "/process" {
                println!("Received request from Addin!");

                // Đọc body
                let mut content = String::new();
                if let Err(e) = request.as_reader().read_to_string(&mut content) {
                    println!("Error reading request body: {}", e);
                    let response = Response::from_string(format!("Error reading body: {}", e));
                    request.respond(response).unwrap();
                    continue;
                }
                println!("Raw data = {}", content);

                let parsed: IncomingData = match serde_json::from_str(&content) {
                    Ok(data) => data,
                    Err(e) => {
                        println!("Error parsing JSON: {}", e);
                        let response = Response::from_string(format!("Error parsing JSON: {}", e));
                        request.respond(response).unwrap();
                        continue;
                    }
                };
                println!("Lat = {}, Lon = {}", parsed.lat, parsed.lng);

                // Clone app_state để sử dụng trong async block
                let state_clone = Arc::clone(&app_state);
                
                // Tạo channel để đợi kết quả
                let (tx, rx) = tokio::sync::oneshot::channel();
                
                // Thêm request vào pending
                {
                    if let Ok(mut pending) = state_clone.pending_requests.lock() {
                        pending.push(tx);
                    }
                }
                
                // Set processing state
                {
                    if let Ok(mut processing) = state_clone.is_processing.lock() {
                        *processing = true;
                    }
                }

                // Gửi sự kiện đến frontend để cập nhật UI
                {
                    if let Ok(window_lock) = state_clone.window.lock() {
                        if let Some(window) = &*window_lock {
                            let _ = window.emit("update-processing-state", true);
                        }
                    }
                }
                
                // Tạo runtime cho async function
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                let response_json = rt.block_on(async {
                    // Lấy cấu hình API hiện tại
                    let config = {
                        if let Ok(config_lock) = state_clone.api_config.lock() {
                            config_lock.clone()
                        } else {
                            ApiConfig::default()
                        }
                    };

                    let result = if !config.custom_url.is_empty() {
                        call_custom_api(parsed.lat, parsed.lng, &config.custom_url).await
                    } else {
                        Err("Custom URL not configured".into())
                    };

                    match result {
                        Ok(result) => {
                            // Lưu dữ liệu mới nhất vào state
                            if let Ok(mut latest_data) = state_clone.latest_data.lock() {
                                *latest_data = Some(result.clone());
                            }
                            
                            // Gửi sự kiện đến frontend để cập nhật kết quả
                            if let Ok(window_lock) = state_clone.window.lock() {
                                if let Some(window) = &*window_lock {
                                    let _ = window.emit("update-result", &result);
                                }
                            }
                            
                            result
                        },
                        Err(e) => {
                            println!("Error calling API: {}", e);
                            // Gửi sự kiện lỗi đến frontend
                            if let Ok(window_lock) = state_clone.window.lock() {
                                if let Some(window) = &*window_lock {
                                    let _ = window.emit("show-error", format!("API Error: {}", e));
                                }
                            }
                            ExampleResult {
                                status: "error".into(),
                                address: format!("API Error: {}", e),
                                ..Default::default()
                            }
                        }
                    }
                });

                // Clear processing state
                {
                    if let Ok(mut processing) = state_clone.is_processing.lock() {
                        *processing = false;
                    }
                }

                // Gửi sự kiện đến frontend để cập nhật UI
                {
                    if let Ok(window_lock) = state_clone.window.lock() {
                        if let Some(window) = &*window_lock {
                            let _ = window.emit("update-processing-state", false);
                        }
                    }
                }

                let resp_text = match serde_json::to_string(&response_json) {
                    Ok(text) => text,
                    Err(e) => {
                        println!("Error serializing response: {}", e);
                        format!("{{\"status\":\"error\",\"address\":\"Serialization error: {}\"}}", e)
                    }
                };

                let response = Response::from_string(resp_text)
                    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());

                if let Err(e) = request.respond(response) {
                    println!("Error sending response: {}", e);
                }
                continue;
            }

            // Handle other routes
            request
                .respond(Response::from_string("Invalid route"))
                .unwrap();
        }
    });
}

// ==================== TAURI COMMANDS ====================

// Command để mở rộng cửa sổ
#[tauri::command]
async fn expand_window(window: tauri::WebviewWindow) -> Result<(), String> {
    println!("Expanding window");
    
    window.set_size(tauri::Size::Logical(tauri::LogicalSize { 
        width: 800.0, 
        height: 600.0 
    })).map_err(|e| e.to_string())?;
    
    window.center().map_err(|e| e.to_string())?;
    window.set_always_on_top(true).map_err(|e| e.to_string())?;
    
    Ok(())
}

// Command để thu nhỏ cửa sổ
#[tauri::command]
async fn collapse_window(window: tauri::WebviewWindow) -> Result<(), String> {
    println!("Collapsing window");
    
    window.set_size(tauri::Size::Logical(tauri::LogicalSize { 
        width: 60.0, 
        height: 60.0 
    })).map_err(|e| e.to_string())?;
    
    // Đặt vị trí góc trên bên phải
    if let Ok(monitor) = window.primary_monitor() {
        if let Some(monitor) = monitor {
            let screen_size = monitor.size();
            let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                x: screen_size.width as f64 - 80.0,
                y: 20.0,
            }));
        }
    }
    
    Ok(())
}

// Command để lấy vị trí cửa sổ
#[tauri::command]
async fn get_window_position(window: tauri::WebviewWindow) -> Result<(f64, f64), String> {
    let position = window
        .inner_position()
        .map_err(|e| e.to_string())?;
    
    Ok((position.x as f64, position.y as f64))
}

// Command để đặt vị trí cửa sổ
#[tauri::command]
async fn set_window_position(window: tauri::WebviewWindow, x: f64, y: f64) -> Result<(), String> {
    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }))
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

// Command để lấy dữ liệu mới nhất
#[tauri::command]
fn get_latest_data(state: tauri::State<Arc<AppState>>) -> Option<ExampleResult> {
    if let Ok(data) = state.latest_data.lock() {
        data.clone()
    } else {
        None
    }
}

// Command để lấy cấu hình API
#[tauri::command]
fn get_api_config(state: tauri::State<Arc<AppState>>) -> Result<ApiConfig, String> {
    if let Ok(config) = state.api_config.lock() {
        Ok(config.clone())
    } else {
        Err("Failed to get API config".to_string())
    }
}

// Command để cập nhật cấu hình API
#[tauri::command]
fn update_api_config(new_config: ApiConfig, state: tauri::State<Arc<AppState>>) -> Result<(), String> {
    if let Ok(mut config) = state.api_config.lock() {
        *config = new_config.clone();
        
        // Lưu cấu hình vào file
        if let Err(e) = save_config(&new_config) {
            println!("Error saving config: {}", e);
            return Err(format!("Failed to save config: {}", e));
        }
        
        Ok(())
    } else {
        Err("Failed to update API config".to_string())
    }
}

// Command để lấy trạng thái processing
#[tauri::command]
fn get_processing_state(state: tauri::State<Arc<AppState>>) -> bool {
    if let Ok(processing) = state.is_processing.lock() {
        *processing
    } else {
        false
    }
}

#[tauri::command]
async fn open_map_view(
    window: tauri::WebviewWindow,
    lat: f64,
    lng: f64,
    map_type: String,
    point_id: String,
) -> Result<(), String> {
    println!("Opening map view: {} at ({}, {}) for point {}", map_type, lat, lng, point_id);
    
    let url = match map_type.as_str() {
        "google" => format!("https://www.google.com/maps?q={},{}", lat, lng),
        "openstreetmap" => format!("https://www.openstreetmap.org/?mlat={}&mlon={}&zoom=17", lat, lng),
        _ => return Err("Invalid map type".to_string()),
    };
    
    // Tạo window ID duy nhất cho mỗi map view
    let window_id = format!("map_view_{}_{}", point_id, map_type);
    
    // Tạo window mới cho map view
    let map_window = tauri::WebviewWindowBuilder::new(
        &window,
        &window_id,
        tauri::WebviewUrl::External(url.parse().unwrap()),
    )
    .title(&format!("{} - {} ({}, {})", map_type, point_id, lat, lng))
    .inner_size(800.0, 600.0)
    .min_inner_size(400.0, 300.0)
    .build()
    .map_err(|e| e.to_string())?;
    
    Ok(())
}

// Command để mở nhiều map view cùng lúc
#[tauri::command]
async fn open_multiple_map_views(
    window: tauri::WebviewWindow,
    lat: f64,
    lng: f64,
    point_id: String,
) -> Result<(), String> {
    println!("Opening multiple map views for point {} at ({}, {})", point_id, lat, lng);
    
    // Mở cả Google Maps và OpenStreetMap cùng lúc
    let map_types = vec!["google", "openstreetmap"];
    
    for map_type in map_types {
        let url = match map_type {
            "google" => format!("https://www.google.com/maps?q={},{}", lat, lng),
            "openstreetmap" => format!("https://www.openstreetmap.org/?mlat={}&mlon={}&zoom=17", lat, lng),
            _ => continue,
        };
        
        let window_id = format!("map_view_{}_{}", point_id, map_type);
        
        let _ = tauri::WebviewWindowBuilder::new(
            &window,
            &window_id,
            tauri::WebviewUrl::External(url.parse().unwrap()),
        )
        .title(&format!("{} - {} ({}, {})", map_type, point_id, lat, lng))
        .inner_size(800.0, 600.0)
        .min_inner_size(400.0, 300.0)
        .build();
    }
    
    Ok(())
}



impl Default for ExampleResult {
    fn default() -> Self {
        Self {
            status: "error".into(),
            address: "".into(),

            poi_vn: None,
            poi_en: None,
            poi_ex: None,

            r#type: None,
            sub_type: None,
            poi_st_sd: None,
            room: None,
            house_num: None,
            buaname: None,
            st_name: None,
            sub_com: None,
            phone: None,
            fax: None,
            web: None,
            mail: None,
            brandname: None,
            import: None,
            status_detail: None,
            note: None,
            done: None,
            update_: None,
            source: None,
            gen_type: None,
            perform: None,
            dup: None,
            explain: None,
            classify: None,
            dtrend: None,
            google_id: None,
            be_id: None,
            plus_code: None,
            latitude: None,
            longitude: None,
        }
    }
}

// Implement Default cho ApiConfig
impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            custom_url: "".to_string(),
            opacity: 0.8,
        }
    }
}

fn main() {
    // Load cấu hình từ file khi khởi động
    let initial_config = load_config();

    let app_state = Arc::new(AppState {
        window: Arc::new(Mutex::new(None)),
        latest_data: Arc::new(Mutex::new(None)),
        pending_requests: Arc::new(Mutex::new(Vec::new())),
        api_config: Arc::new(Mutex::new(initial_config)),
        is_processing: Arc::new(Mutex::new(false)),
    });

    let state_clone = Arc::clone(&app_state);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            // Tạo system tray menu
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide Window", true, None::<&str>)?;
            let set_url_item = MenuItem::with_id(app, "set_url", "Set Custom URL", true, None::<&str>)?;
            let opacity_item = MenuItem::with_id(app, "opacity", "Set Opacity", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            
            // Tạo separator items
            let separator1 = MenuItem::with_id(app, "sep1", "---", false, None::<&str>)?;
            let separator2 = MenuItem::with_id(app, "sep2", "---", false, None::<&str>)?;
            
            let menu = Menu::with_items(app, &[
                &show_item,
                &hide_item,
                &separator1,
                &set_url_item,
                &opacity_item,
                &separator2,
                &quit_item,
            ])?;

            // Tạo system tray
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    let window = app.get_webview_window("main").unwrap();
                    match event.id.as_ref() {
                        "show" => {
                            println!("Show window menu item clicked");
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        "hide" => {
                            println!("Hide window menu item clicked");
                            let _ = window.hide();
                        }
                        "set_url" => {
                            println!("Set URL menu item clicked");
                            let _ = window.emit("open-url-input", ());
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        "opacity" => {
                            println!("Opacity menu item clicked");
                            let _ = window.emit("open-opacity-selector", ());
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        "quit" => {
                            println!("Quit menu item clicked");
                            app.exit(0);
                        }
                        "sep1" | "sep2" => {
                            // Bỏ qua separator items
                        }
                        _ => {
                            println!("Unknown menu item: {:?}", event.id);
                        }
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            println!("Left click on tray icon");
                            let app = tray.app_handle();
                            let window = app.get_webview_window("main").unwrap();
                            if window.is_visible().unwrap() {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            let main_window = app.get_webview_window("main").unwrap();
            
            // Cấu hình window floating - Ẩn khỏi taskbar
            main_window.set_always_on_top(true).unwrap();
            main_window.set_decorations(false).unwrap();
            main_window.set_skip_taskbar(true).unwrap();
            
            // Đặt kích thước nhỏ (icon)
            main_window.set_size(tauri::Size::Logical(tauri::LogicalSize { 
                width: 60.0, 
                height: 60.0 
            })).unwrap();
            
            // Đặt vị trí góc trên bên phải
            if let Ok(monitor) = main_window.primary_monitor() {
                if let Some(monitor) = monitor {
                    let screen_size = monitor.size();
                    let _ = main_window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                        x: screen_size.width as f64 - 80.0,
                        y: 20.0,
                    }));
                }
            }

            // Ẩn window khi khởi động, chỉ hiện system tray
            let _ = main_window.hide();

            // Lưu window reference vào state
            if let Ok(mut window_lock) = state_clone.window.lock() {
                *window_lock = Some(main_window);
            }
            
            start_local_server(state_clone);
            Ok(())
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_latest_data, 
            expand_window,
            collapse_window,
            get_window_position,  
            set_window_position,
            get_api_config,
            update_api_config,
            get_processing_state,
            open_map_view,
            open_multiple_map_views
        ])
        .run(generate_context!())
        .expect("error while running Tauri application");
}