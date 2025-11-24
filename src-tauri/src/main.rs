#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;
use tiny_http::{Header, Method, Response, Server};
use tauri::{Manager, Window, Emitter, WebviewWindow};
use reqwest;

#[derive(Deserialize)]
struct IncomingData {
    lat: f64,
    lng: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ExampleResult {
    pub status: String,
    pub address: String,
    pub province: String,
    pub district: String,
    pub ward: String,

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
    pub dine: Option<String>,
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
}

// Struct để parse response từ Goong API
#[derive(Debug, Deserialize)]
struct GoongResponse {
    results: Vec<GoongResult>,
    status: String,
}

#[derive(Debug, Deserialize)]
struct GoongResult {
    formatted_address: Option<String>,
    address_components: Option<Vec<AddressComponent>>,
    name: Option<String>,
    types: Option<Vec<String>>,
    place_id: Option<String>,
    compound: Option<Compound>,
    address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddressComponent {
    long_name: String,
    short_name: String,
}

#[derive(Debug, Deserialize)]
struct Compound {
    district: Option<String>,
    commune: Option<String>,
    province: Option<String>,
}

// State để lưu trữ window và dữ liệu mới nhất
struct AppState {
    window: Arc<Mutex<Option<WebviewWindow>>>, 
    latest_data: Arc<Mutex<Option<ExampleResult>>>,
}

// Hàm gọi API Goong.io
async fn call_goong_api(lat: f64, lng: f64) -> Result<ExampleResult, Box<dyn std::error::Error>> {
    let api_key = "T4B6StzJYTsTEyxA0u9I01593mA1yclUffVMODpx"; // Thay YOUR_API_KEY bằng API key thực tế
    let url = format!(
        "https://rsapi-test.goong.io/v2/geocode?latlng={},{}&limit=5&api_key={}&has_deprecated_administrative_unit=true",
        lat, lng, api_key
    );

    println!("Calling Goong API: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let response_text = response.text().await?;
    println!("Goong API response: {}", response_text);

    let goong_response: GoongResponse = serde_json::from_str(&response_text)?;
    
    if goong_response.status != "OK" {
        return Err(format!("Goong API returned status: {}", goong_response.status).into());
    }

    // Lấy kết quả đầu tiên
    if let Some(first_result) = goong_response.results.get(0) {
        println!("Processing first result: {:?}", first_result);

        // Sử dụng compound field để lấy thông tin hành chính
        let (province, district, ward) = if let Some(compound) = &first_result.compound {
            (
                compound.province.clone().unwrap_or_default(),
                compound.district.clone().unwrap_or_default(),
                compound.commune.clone().unwrap_or_default(),
            )
        } else {
            // Fallback: parse từ address_components nếu không có compound
            let mut province = String::new();
            let mut district = String::new();
            let mut ward = String::new();

            if let Some(components) = &first_result.address_components {
                // Dựa vào thứ tự trong mảng address_components
                match components.len() {
                    4 => {
                        // Format: [số nhà, phường, quận, thành phố]
                        ward = components.get(1).map(|c| c.long_name.clone()).unwrap_or_default();
                        district = components.get(2).map(|c| c.long_name.clone()).unwrap_or_default();
                        province = components.get(3).map(|c| c.long_name.clone()).unwrap_or_default();
                    }
                    3 => {
                        // Format: [phường, quận, thành phố]
                        ward = components.get(0).map(|c| c.long_name.clone()).unwrap_or_default();
                        district = components.get(1).map(|c| c.long_name.clone()).unwrap_or_default();
                        province = components.get(2).map(|c| c.long_name.clone()).unwrap_or_default();
                    }
                    _ => {
                        // Try to find by name pattern
                        for component in components {
                            let name = &component.long_name;
                            if name.contains("Hà Nội") || name.contains("Hồ Chí Minh") || name.contains("Đà Nẵng") {
                                province = name.clone();
                            } else if name.contains("Quận") || name.contains("Huyện") {
                                district = name.clone();
                            } else if name.contains("Phường") || name.contains("Xã") {
                                ward = name.clone();
                            }
                        }
                    }
                }
            }

            (province, district, ward)
        };

        // Extract house number and street name from name or formatted_address
        let (house_num, st_name) = extract_address_parts(
            first_result.name.as_deref().unwrap_or(""),
            first_result.formatted_address.as_deref().unwrap_or("")
        );

        let result = ExampleResult {
            status: "ok".into(),
            address: first_result.formatted_address.clone().unwrap_or_else(|| "Unknown address".into()),
            province: clean_province_name(&province),
            district: clean_district_name(&district),
            ward: clean_ward_name(&ward),

            poi_vn: first_result.name.clone(),
            poi_en: None,
            poi_ex: None,

            r#type: first_result.types.as_ref().and_then(|types| {
                if types.is_empty() {
                    Some("address".into())
                } else {
                    types.get(0).cloned()
                }
            }),
            sub_type: None,
            poi_st_sd: Some("Standard POI".into()),

            room: None,
            house_num: if house_num.is_empty() { None } else { Some(house_num) },
            buaname: None,
            st_name: if st_name.is_empty() { None } else { Some(st_name) },
            sub_com: None,

            phone: None,
            fax: None,
            web: None,
            mail: None,

            brandname: None,
            import: None,
            status_detail: Some("active".into()),
            note: None,
            dine: None,
            update_: Some("2025-01-01".into()),
            source: Some("goong".into()),
            gen_type: Some("public".into()),
            perform: None,
            dup: None,
            explain: None,
            classify: None,
            dtrend: None,

            google_id: first_result.place_id.clone(),
            be_id: None,
        };

        println!("Processed result: {:?}", result);
        Ok(result)
    } else {
        println!("No results found in Goong API response");
        Ok(ExampleResult {
            status: "error".into(),
            address: "No results found".into(),
            province: "".into(),
            district: "".into(),
            ward: "".into(),

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
            status_detail: Some("no_results".into()),
            note: None,
            dine: None,
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
        })
    }
}

// Hàm trích xuất số nhà và tên đường từ name và formatted_address
fn extract_address_parts(name: &str, formatted_address: &str) -> (String, String) {
    let mut house_num = String::new();
    let mut st_name = String::new();

    // Ưu tiên phân tích từ name trước
    if !name.is_empty() {
        let parts: Vec<&str> = name.split_whitespace().collect();
        if !parts.is_empty() {
            if let Some(first_part) = parts.get(0) {
                if first_part.chars().all(|c| c.is_ascii_digit()) {
                    house_num = first_part.to_string();
                    st_name = parts[1..].join(" ");
                } else {
                    st_name = name.to_string();
                }
            }
        }
    }

    // Nếu không tìm thấy từ name, thử từ formatted_address
    if house_num.is_empty() && !formatted_address.is_empty() {
        let parts: Vec<&str> = formatted_address.split(',').collect();
        if let Some(first_part) = parts.get(0) {
            let first_part = first_part.trim();
            let sub_parts: Vec<&str> = first_part.split_whitespace().collect();
            if !sub_parts.is_empty() && sub_parts[0].chars().all(|c| c.is_ascii_digit()) {
                house_num = sub_parts[0].to_string();
                if sub_parts.len() > 1 {
                    st_name = sub_parts[1..].join(" ");
                }
            }
        }
    }

    (house_num, st_name)
}

// Hàm làm sạch tên tỉnh/thành phố
fn clean_province_name(province: &str) -> String {
    province
        .replace("Thành phố ", "")
        .replace("Tỉnh ", "")
        .trim()
        .to_string()
}

// Hàm làm sạch tên quận/huyện
fn clean_district_name(district: &str) -> String {
    district
        .replace("Quận ", "")
        .replace("Huyện ", "")
        .trim()
        .to_string()
}

// Hàm làm sạch tên phường/xã
fn clean_ward_name(ward: &str) -> String {
    ward
        .replace("Phường ", "")
        .replace("Xã ", "")
        .trim()
        .to_string()
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
                
                // Tạo runtime cho async function
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                let response_json = rt.block_on(async {
                    match call_goong_api(parsed.lat, parsed.lng).await {
                        Ok(result) => {
                            // Lưu dữ liệu mới nhất vào state
                            if let Ok(mut latest_data) = state_clone.latest_data.lock() {
                                *latest_data = Some(result.clone());
                            }
                            
                            // Gửi sự kiện đến frontend
                            if let Ok(window_lock) = state_clone.window.lock() {
                                if let Some(window) = &*window_lock {
                                    let _ = window.emit("new-data", &result); 
                                    println!("Emitted new-data event to frontend");
                                }
                            }
                            result
                        },
                        Err(e) => {
                            println!("Error calling Goong API: {}", e);
                            let error_result = ExampleResult {
                                status: "error".into(),
                                address: format!("API Error: {}", e),
                                province: "".into(),
                                district: "".into(),
                                ward: "".into(),
                                ..Default::default()
                            };
                            
                            // Gửi sự kiện lỗi đến frontend
                            if let Ok(window_lock) = state_clone.window.lock() {
                                if let Some(window) = &*window_lock {
                                    let _ = window.emit("new-data", &error_result);
                                }
                            }
                            error_result
                        }
                    }
                });

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

// Implement Default cho ExampleResult để dễ xử lý lỗi
impl Default for ExampleResult {
    fn default() -> Self {
        Self {
            status: "error".into(),
            address: "".into(),
            province: "".into(),
            district: "".into(),
            ward: "".into(),
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
            dine: None,
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
        }
    }
}

#[tauri::command]
fn get_latest_data(state: tauri::State<Arc<AppState>>) -> Option<ExampleResult> {
    if let Ok(data) = state.latest_data.lock() {
        data.clone()
    } else {
        None
    }
}

fn main() {
    let app_state = Arc::new(AppState {
        window: Arc::new(Mutex::new(None)),
        latest_data: Arc::new(Mutex::new(None)),
    });

    let state_clone = Arc::clone(&app_state);

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![get_latest_data])
        .setup(move |app| {
            let main_window = app.get_webview_window("main").unwrap();
            
            // Lưu window reference vào state
            if let Ok(mut window_lock) = state_clone.window.lock() {
                *window_lock = Some(main_window);
            }
            
            start_local_server(state_clone);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}