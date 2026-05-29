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
use reqwest;
use chrono::{DateTime, FixedOffset, Utc};
use chrono::offset::TimeZone;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MapConfig {
    google: bool,
    openstreetmap: bool,
    bing: bool,
    streetviewvn: bool,
    mapillary: bool,
    vietbando: bool,
    herewego: bool,
    wikimapia: bool,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            google: true,
            openstreetmap: true,
            bing: false,
            streetviewvn: false,
            mapillary: false,
            vietbando: false,
            herewego: false,
            wikimapia: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiConfig {
    base_url: String,
    opacity: f64,
    maps: MapConfig,
    default_perform: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: "".to_string(),
            opacity: 0.8,
            maps: MapConfig::default(),
            default_perform: "".to_string(),
        }
    }
}


// State để lưu trữ window và dữ liệu mới nhất
struct AppState {
    window: Arc<Mutex<Option<tauri::WebviewWindow>>>,
    latest_data: Arc<Mutex<Option<ExampleResult>>>,
    pending_requests: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<ExampleResult>>>>,
    api_config: Arc<Mutex<ApiConfig>>,
    is_processing: Arc<Mutex<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum MapType {
    Google,
    OpenStreetMap,
    Bing,
    StreetViewVN,
    Mapillary,
    Vietbando,
    HereWeGo,
    Wikimapia,
}

impl MapType {
    fn as_str(&self) -> &'static str {
        match self {
            MapType::Google => "google",
            MapType::OpenStreetMap => "openstreetmap",
            MapType::Bing => "bing",
            MapType::StreetViewVN => "streetviewvn",
            MapType::Mapillary => "mapillary",
            MapType::Vietbando => "vietbando",
            MapType::HereWeGo => "herewego",
            MapType::Wikimapia => "wikimapia",
        }
    }
    
    fn get_url(&self, lat: f64, lng: f64) -> String {
        match self {
            MapType::Google => format!("https://www.google.com/maps?q={},{}", lat, lng),
            MapType::OpenStreetMap => format!("https://www.openstreetmap.org/?mlat={}&mlon={}&zoom=17", lat, lng),
            MapType::Bing => format!("https://www.bing.com/maps?cp={}~{}&lvl=11&style=r", lat, lng),
            MapType::StreetViewVN => format!("https://www.streetview.vn/?lat={}&lng={}", lat, lng),
            MapType::Mapillary => format!("https://www.mapillary.com/app/?lat={}&lng={}&z=16.53889080546365&menu=false", lat, lng),
            MapType::Vietbando => "http://maps.vietbando.com/maps/".to_string(),
            MapType::HereWeGo => format!("https://wego.here.com/?map={},{},10", lat, lng),
            MapType::Wikimapia => format!("https://wikimapia.org/#lang=en&lat={}&lon={}&z=12&m=w", lat, lng),
        }
    }
    
    fn get_icon(&self) -> &'static str {
        match self {
            MapType::Google => "🗺️",
            MapType::OpenStreetMap => "🌍",
            MapType::Bing => "🅱️",
            MapType::StreetViewVN => "👁️",
            MapType::Mapillary => "📷",
            MapType::Vietbando => "🇻🇳",
            MapType::HereWeGo => "📍",
            MapType::Wikimapia => "📖",
        }
    }
    
    fn get_name(&self) -> &'static str {
        match self {
            MapType::Google => "Google Maps",
            MapType::OpenStreetMap => "OpenStreetMap",
            MapType::Bing => "Bing Maps",
            MapType::StreetViewVN => "StreetView.vn",
            MapType::Mapillary => "Mapillary",
            MapType::Vietbando => "Vietbando",
            MapType::HereWeGo => "Here WeGo",
            MapType::Wikimapia => "Wikimapia",
        }
    }
}

/// Tính khoảng cách (mét) giữa hai điểm theo công thức Haversine
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371000.0; // bán kính Trái Đất (m)
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos()
        * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
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

// Hàm gọi Geocode API để lấy thông tin cơ bản và place_id
async fn call_geocode_api(lat: f64, lng: f64, base_url: &str) -> Result<ExampleResult, Box<dyn std::error::Error>> {
    let url = format!("{}/geocode?latlng={},{}", base_url, lat, lng);
    println!("Calling Geocode API: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("Geocode API request failed with status: {}", response.status()).into());
    }

    let response_text = response.text().await?;
    println!("Geocode API response: {}", response_text);

    // Parse response từ Google Geocoding API
    let geocode_response: serde_json::Value = serde_json::from_str(&response_text)?;
    
    // Tạo ExampleResult từ dữ liệu geocode
    let mut result = parse_google_geocoding_response(geocode_response);
    
    Ok(result)
}

// Hàm gọi Placedetails API để lấy thêm thông tin chi tiết
async fn call_placedetails_api(place_id: &str, base_url: &str) -> Result<ExampleResult, Box<dyn std::error::Error>> {
    let url = format!("{}/placedetails?place_id={}", base_url, place_id);
    println!("Calling Placedetails API: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("Placedetails API request failed with status: {}", response.status()).into());
    }

    let response_text = response.text().await?;
    println!("Placedetails API response: {}", response_text);

    // Parse response từ Google Places Details API
    let placedetails_response: serde_json::Value = serde_json::from_str(&response_text)?;
    
    // Tạo ExampleResult từ dữ liệu placedetails
    let result = parse_placedetails_response(placedetails_response);
    
    Ok(result)
}

pub fn parse_google_geocoding_response(response: Value) -> ExampleResult {
    let mut result = ExampleResult {
        status: "A".to_string(), 
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
        source: Some("Googlemap".to_string()),
        gen_type: None,
        perform: None,
        
        dup: None,
        explain: Some("4".to_string()),
        classify: Some("P".to_string()),
        dtrend: None,
        google_id: None,
        be_id: None,
        plus_code: None,
        latitude: None,
        longitude: None,
    };

    if let Some(status) = response["status"].as_str() {
        if status != "OK" {
            result.status = "D".to_string(); 
            result.status_detail = Some(format!("Google API error: {}", status));
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

            if is("premise") || is("point_of_interest") {
                result.poi_vn = Some(long.clone());
            }
            if is("street_number") {
                let mut house = long.clone();
                // Loại bỏ tất cả các số 0 ở đầu
                let trimmed = house.trim_start_matches('0');
                if trimmed.is_empty() {
                    // Trường hợp chuỗi chỉ toàn số 0 (ví dụ "00" -> "0")
                    house = "0".to_string();
                } else {
                    house = trimmed.to_string();
                }
                result.house_num = Some(house);
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
                // Chuẩn hóa unicode trước khi kiểm tra
                let normalized_long = unicode_normalization::UnicodeNormalization::nfc(long.chars())
                    .collect::<String>();
                
                let cleaned_name = if normalized_long.starts_with("Đường ") {
                    normalized_long.replacen("Đường ", "", 1)
                } else if normalized_long.starts_with("đường ") {
                    normalized_long.replacen("đường ", "", 1)
                } else if normalized_long.starts_with("Đ. ") {
                    normalized_long.replacen("Đ. ", "", 1)
                } else {
                    normalized_long
                };
                result.st_name = Some(cleaned_name);
            }
        }
    }

    result.phone = first["formatted_phone_number"].as_str().map(|s| s.to_string());
    result.web = first["website"].as_str().map(|s| s.to_string());
    result.google_id = first["place_id"].as_str().map(|s| s.to_string());

    result
}


pub fn parse_placedetails_response(response: Value) -> ExampleResult {
    let mut result = ExampleResult::default();

    if let Some(status) = response["status"].as_str() {
        if status != "OK" {
            result.status = "D".to_string();
            result.status_detail = Some(format!("Places API error: {}", status));
            return result;
        }
    }

    let detail = &response["result"];

    if let Some(name) = detail["name"].as_str() {
        result.poi_vn = Some(name.to_string());
    }

    if let Some(phone) = detail["formatted_phone_number"].as_str() {
        result.phone = Some(phone.to_string());
    }

    if let Some(web) = detail["website"].as_str() {
        result.web = Some(web.to_string());
    }

    // Lấy tọa độ từ geometry.location (nếu có)
    if let Some(loc) = detail["geometry"]["location"].as_object() {
        result.latitude = loc.get("lat").and_then(|v| v.as_f64());
        result.longitude = loc.get("lng").and_then(|v| v.as_f64());
    }

    // Parse type và sub_type từ types array
    if let Some(types_array) = detail["types"].as_array() {
        // Tạo mapping từ bảng
        let type_mapping: Vec<(&str, &str)> = vec![
            ("accounting", "Bu6"),
            ("airport", "Tran5"),
            ("amusement_park", "Rec5"),
            ("aquarium", "Shop2"),
            ("art_gallery", "Ent1"),
            ("atm", "Bu1"),
            ("bakery", "FD2"),
            ("bank", "Bu2"),
            ("bar", "FD1"),
            ("beauty_salon", "Shop2"),
            ("bicycle_store", "Shop2"),
            ("book_store", "Se1"),
            ("bowling_alley", ""),
            ("bus_station", "Tran4"),
            ("cafe", "FD3"),
            ("campground", "Si1"),
            ("car_dealer", "Au1"),
            ("car_rental", "Au1"),
            ("car_repair", "Au4"),
            ("car_wash", "Au4"),
            ("casino", ""),
            ("cemetery", "Si5"),
            ("church", "Si2"),
            ("city_hall", ""),
            ("clothing_store", "Shop2"),
            ("convenience_store", "Shop2"),
            ("courthouse", "Gov2"),
            ("dentist", "Eme2"),
            ("department_store", "Shop2"),
            ("doctor", "Eme6"),
            ("drugstore", "Eme1"),
            ("electrician", "Shop2"),
            ("electronics_store", "Shop2"),
            ("embassy", "Gov6"),
            ("fire_station", "Eme4"),
            ("florist", "Shop2"),
            ("funeral_home", "Si5"),
            ("furniture_store", "Shop2"),
            ("gas_station", "Au3"),
            ("gym", "Rec8"),
            ("hair_care", "Shop2"),
            ("hardware_store", "Shop2"),
            ("hindu_temple", "Si4"),
            ("home_goods_store", "Shop2"),
            ("hospital", "Eme3"),
            ("insurance_agency", "Bu6"),
            ("jewelry_store", "Shop2"),
            ("laundry", "Shop2"),
            ("lawyer", "Bu6"),
            ("library", "Se1"),
            ("light_rail_station", "Tran1"),
            ("liquor_store", "FD2"),
            ("local_government_office", "Gov7"),
            ("locksmith", ""),
            ("lodging", "Lod1, Lod2, Lod3"),
            ("meal_delivery", ""),
            ("meal_takeaway", ""),
            ("mosque", "Si2"),
            ("movie_rental", ""),
            ("movie_theater", "Ent5"),
            ("moving_company", "Bu6"),
            ("museum", "Ent2"),
            ("night_club", "FD1"),
            ("painter", ""),
            ("park", "Rec5"),
            ("parking", "Au2"),
            ("pet_store", ""),
            ("pharmacy", "Eme1"),
            ("physiotherapist", "Eme6"),
            ("plumber", ""),
            ("police", "Eme5"),
            ("post_office", "Se3"),
            ("primary_school", "Edu1"),
            ("real_estate_agency", "Bu6"),
            ("restaurant", "FD2"),
            ("roofing_contractor", "Bu6"),
            ("rv_park", "Rec5"),
            ("school", ""),
            ("secondary_school", "Edu2"),
            ("shoe_store", "Shop2"),
            ("shopping_mall", "Shop1"),
            ("spa", "Rec4"),
            ("stadium", "Rec3"),
            ("storage", ""),
            ("store", "Shop2"),
            ("subway_station", "Tran1"),
            ("supermarket", "Shop4"),
            ("synagogue", "Si2"),
            ("taxi_stand", ""),
            ("tourist_attraction", "Si1"),
            ("train_station", "Tran1"),
            ("transit_station", "Tran4"),
            ("travel_agency", "Se2"),
            ("university", "Edu4"),
            ("veterinary_care", "Eme6"),
            ("zoo", "Ent6"),
        ];

        // Tìm type_google đầu tiên trong mảng types mà có trong mapping
        for google_type in types_array {
            if let Some(google_type_str) = google_type.as_str() {
                if let Some(&(_, sub_type)) = type_mapping.iter()
                    .find(|&&(type_name, _)| type_name == google_type_str) {
                    
                    if !google_type_str.is_empty() {
                        result.r#type = Some(google_type_str.to_string());
                    }
                    
                    if !sub_type.is_empty() {
                        result.sub_type = Some(sub_type.to_string());
                    }
                    
                    // Chỉ lấy type đầu tiên tìm thấy
                    break;
                }
            }
        }
    }

    result
}


async fn call_custom_api(lat: f64, lng: f64, base_url: &str, default_perform: &str) -> Result<ExampleResult, Box<dyn std::error::Error>> {
    let mut result = call_geocode_api(lat, lng, base_url).await?;
    
    if !default_perform.is_empty() {
        result.perform = Some(default_perform.to_string());
    }

    if let Some(place_id) = &result.google_id {
        match call_placedetails_api(place_id, base_url).await {
            Ok(details) => {
                // --- KIỂM TRA KHOẢNG CÁCH ---
                if let (Some(detail_lat), Some(detail_lng)) = (details.latitude, details.longitude) {
                    let distance = haversine_distance(lat, lng, detail_lat, detail_lng);
                    println!("Distance between original ({},{}) and place details ({},{}): {:.2} meters",
                             lat, lng, detail_lat, detail_lng, distance);
                    if distance > 50.0 {
                        result.status = "D".to_string();
                        result.status_detail = Some(format!("Không có điểm phù hợp (khoảng cách {:.1}m > 50m)", distance));
                    }
                } else {
                    println!("Warning: Place Details does not contain coordinates");
                }
                if let Some(poi_vn) = details.poi_vn {
                    result.poi_vn = Some(poi_vn);
                }
                if let Some(phone) = details.phone {
                    result.phone = Some(phone);
                }
                if let Some(web) = details.web {
                    result.web = Some(web);
                }
                // Thêm parse cho type và sub_type
                if let Some(type_val) = details.r#type {
                    result.r#type = Some(type_val);
                }
                if let Some(sub_type) = details.sub_type {
                    result.sub_type = Some(sub_type);
                }
            }
            Err(e) => {
                println!("Error calling placedetails API: {}", e);
                result.status_detail = Some(format!("Placedetails API error: {}", e));
            }
        }
    }

    // Thêm ngày cập nhật (GMT+7) với định dạng dd/mm/yyyy
    let gmt_plus_7 = FixedOffset::east_opt(7 * 3600).unwrap(); // GMT+7
    let now_utc = Utc::now();
    let now_gmt7 = gmt_plus_7.from_utc_datetime(&now_utc.naive_utc());
    
    result.update_ = Some(now_gmt7.format("%d/%m/%Y").to_string());


    Ok(result)
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

                let state_clone = Arc::clone(&app_state);
                
                let (tx, rx) = tokio::sync::oneshot::channel();
                
                {
                    if let Ok(mut pending) = state_clone.pending_requests.lock() {
                        pending.push(tx);
                    }
                }
                
                {
                    if let Ok(mut processing) = state_clone.is_processing.lock() {
                        *processing = true;
                    }
                }

                {
                    if let Ok(window_lock) = state_clone.window.lock() {
                        if let Some(window) = &*window_lock {
                            let _ = window.emit("update-processing-state", true);
                        }
                    }
                }
                
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                let response_json = rt.block_on(async {
                    let config = {
                        if let Ok(config_lock) = state_clone.api_config.lock() {
                            config_lock.clone()
                        } else {
                            ApiConfig::default()
                        }
                    };

                    let result = if !config.base_url.is_empty() {
                        call_custom_api(parsed.lat, parsed.lng, &config.base_url, &config.default_perform).await
                    } else {
                        Err("Base URL not configured".into())
                    };

                    match result {
                        Ok(result) => {
                            if let Ok(mut latest_data) = state_clone.latest_data.lock() {
                                *latest_data = Some(result.clone());
                            }
                            
                            if let Ok(window_lock) = state_clone.window.lock() {
                                if let Some(window) = &*window_lock {
                                    let _ = window.emit("update-result", &result);
                                }
                            }
                            
                            result
                        },
                        Err(e) => {
                            println!("Error calling API: {}", e);
                            if let Ok(window_lock) = state_clone.window.lock() {
                                if let Some(window) = &*window_lock {
                                    let _ = window.emit("show-error", format!("API Error: {}", e));
                                }
                            }
                            ExampleResult {
                                status: "D".into(), 
                                address: format!("API Error: {}", e),
                                source: Some("Googlemap".to_string()),
                                explain: Some("4-Build_update".to_string()),
                                classify: Some("P-Private".to_string()),
                                ..Default::default()
                            }
                        }
                    }
                });

                {
                    if let Ok(mut processing) = state_clone.is_processing.lock() {
                        *processing = false;
                    }
                }

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
                        format!("{{\"status\":\"D\",\"address\":\"Serialization error: {}\",\"source\":\"Googlemap\",\"explain\":\"4-Build_update\",\"classify\":\"P-Private\"}}", e)
                    }
                };

                let response = Response::from_string(resp_text)
                    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());

                if let Err(e) = request.respond(response) {
                    println!("Error sending response: {}", e);
                }
                continue;
            }

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
        "google" => MapType::Google.get_url(lat, lng),
        "openstreetmap" => MapType::OpenStreetMap.get_url(lat, lng),
        "bing" => MapType::Bing.get_url(lat, lng),
        "streetviewvn" => MapType::StreetViewVN.get_url(lat, lng),
        "mapillary" => MapType::Mapillary.get_url(lat, lng),
        "vietbando" => MapType::Vietbando.get_url(lat, lng),
        "herewego" => MapType::HereWeGo.get_url(lat, lng),
        "wikimapia" => MapType::Wikimapia.get_url(lat, lng),
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

#[tauri::command]
async fn open_selected_maps(
    window: tauri::WebviewWindow,
    lat: f64,
    lng: f64,
    point_id: String,
    map_config: MapConfig,
) -> Result<(), String> {
    println!("Opening selected maps for point {} at ({}, {})", point_id, lat, lng);
    
    let maps_to_open = vec![
        (MapType::Google, map_config.google),
        (MapType::OpenStreetMap, map_config.openstreetmap),
        (MapType::Bing, map_config.bing),
        (MapType::StreetViewVN, map_config.streetviewvn),
        (MapType::Mapillary, map_config.mapillary),
        (MapType::Vietbando, map_config.vietbando),
        (MapType::HereWeGo, map_config.herewego),
        (MapType::Wikimapia, map_config.wikimapia),
    ];
    
    for (map_type, enabled) in maps_to_open {
        if enabled {
            let url = map_type.get_url(lat, lng);
            let window_id = format!("map_view_{}_{}", point_id, map_type.as_str());
            
            let _ = tauri::WebviewWindowBuilder::new(
                &window,
                &window_id,
                tauri::WebviewUrl::External(url.parse().unwrap()),
            )
            .title(&format!("{} - {} ({}, {})", map_type.get_name(), point_id, lat, lng))
            .inner_size(800.0, 600.0)
            .min_inner_size(400.0, 300.0)
            .build();
        }
    }
    
    Ok(())
}

impl Default for ExampleResult {
    fn default() -> Self {
        Self {
            status: "D".into(),
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
            source: Some("Googlemap".to_string()),
            gen_type: None,
            perform: None,
            dup: None,
            explain: Some("4-Build_update".to_string()),
            classify: Some("P-Private".to_string()),
            dtrend: None,
            google_id: None,
            be_id: None,
            plus_code: None,
            latitude: None,
            longitude: None,
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
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide Window", true, None::<&str>)?;
            let set_url_item = MenuItem::with_id(app, "set_url", "Set Base URL", true, None::<&str>)?;
            let opacity_item = MenuItem::with_id(app, "opacity", "Set Opacity", true, None::<&str>)?;
            
            let set_perform_item = MenuItem::with_id(app, "set_perform", "Set Perform Value", true, None::<&str>)?;
            
            let select_maps_item = MenuItem::with_id(app, "select_maps", "Select Maps", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let separator1 = MenuItem::with_id(app, "sep1", "---", false, None::<&str>)?;
            let separator2 = MenuItem::with_id(app, "sep2", "---", false, None::<&str>)?;
            let separator3 = MenuItem::with_id(app, "sep3", "---", false, None::<&str>)?;
            
            let menu = Menu::with_items(app, &[
                &show_item,
                &hide_item,
                &separator1,
                &set_url_item,
                &opacity_item,
                &set_perform_item, 
                &select_maps_item,
                &separator2,
                &quit_item,
            ])?;

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
                        "set_perform" => {
                            println!("Set Perform menu item clicked");
                            let _ = window.emit("open-perform-input", ());
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        "select_maps" => {
                            println!("Select Maps menu item clicked");
                            let _ = window.emit("open-map-selector", ());
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        "quit" => {
                            println!("Quit menu item clicked");
                            app.exit(0);
                        }
                        "sep1" | "sep2" => {
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
            
            main_window.set_always_on_top(true).unwrap();
            main_window.set_decorations(false).unwrap();
            main_window.set_skip_taskbar(true).unwrap();
            
            main_window.set_size(tauri::Size::Logical(tauri::LogicalSize { 
                width: 60.0, 
                height: 60.0 
            })).unwrap();
            
            if let Ok(monitor) = main_window.primary_monitor() {
                if let Some(monitor) = monitor {
                    let screen_size = monitor.size();
                    let _ = main_window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                        x: screen_size.width as f64 - 80.0,
                        y: 20.0,
                    }));
                }
            }

            let _ = main_window.hide();

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
            open_multiple_map_views,
            open_selected_maps,
        ])
        .run(generate_context!())
        .expect("error while running Tauri application");
}
