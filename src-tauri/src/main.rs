#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Read;
use std::thread;
use tiny_http::{Header, Method, Response, Server};
use tauri::Manager;

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

    pub poi_vn: Option<String>,
    pub poi_en: Option<String>,
    pub poi_ex: Option<String>,

    pub r#type: Option<String>,
    pub sub_type: Option<String>,
    pub poi_st_sd: Option<String>,

    pub room: Option<String>,
    pub house_num: Option<String>,
    pub buaname: Option<String>,
    pub st_name: Option<String>,
    pub sub_com: Option<String>,

    pub phone: Option<String>,
    pub fax: Option<String>,
    pub web: Option<String>,
    pub mail: Option<String>,

    pub brandname: Option<String>,
    pub import: Option<String>,
    pub status_detail: Option<String>,  // tránh trùng với "status"
    pub note: Option<String>,
    pub dine: Option<String>,
    pub update_: Option<String>,
    pub source: Option<String>,
    pub gen_type: Option<String>,
    pub perform: Option<String>,
    pub dup: Option<String>,
    pub explain: Option<String>,
    pub classify: Option<String>,
    pub dtrend: Option<String>,

    pub google_id: Option<String>,
    pub be_id: Option<String>,
}


fn start_local_server() {
    thread::spawn(|| {
        let server = Server::http("127.0.0.1:31203").unwrap();
        println!("Tauri server listening on http://127.0.0.1:31203");

        loop {
            // 🔥 PHẢI ĐỂ request LÀ MUT
            let mut request = match server.recv() {
                Ok(rq) => rq,
                Err(e) => {
                    println!("Server error: {}", e);
                    continue;
                }
            };

            if request.method() == &Method::Post && request.url() == "/process" {
                println!("Received request from Addin!");

                // ---- READ BODY ----
                let mut content = String::new();
                request.as_reader().read_to_string(&mut content).unwrap();
                println!("Raw data = {}", content);

                let parsed: IncomingData = serde_json::from_str(&content).unwrap();
                println!("Lat = {}, Lon = {}", parsed.lat, parsed.lng);

                // ---- Example return data ----
                let response_json = ExampleResult {
                    status: "ok".into(),
                    address: "123 Sample Street".into(),
                    province: "Hanoi".into(),
                    district: "Cau Giay".into(),
                    ward: "Dich Vong".into(),

                    poi_vn: Some("Trường Đại học Quốc gia".into()),
                    poi_en: Some("Vietnam National University".into()),
                    poi_ex: None,   // bỏ qua trường này

                    r#type: Some("Education".into()),
                    sub_type: None, // bỏ qua
                    poi_st_sd: Some("Standard POI".into()),

                    room: None,
                    house_num: Some("12A".into()),
                    buaname: None,
                    st_name: Some("Pham Hung".into()),
                    sub_com: None,

                    phone: Some("0123456789".into()),
                    fax: None,
                    web: None,
                    mail: Some("info@example.com".into()),

                    brandname: Some("VNU".into()),
                    import: None,
                    status_detail: Some("active".into()),
                    note: None,
                    dine: None,
                    update_: Some("2025-01-01".into()),
                    source: Some("google".into()),
                    gen_type: Some("public".into()),
                    perform: None,
                    dup: None,
                    explain: None,
                    classify: None,
                    dtrend: None,

                    google_id: None,
                    be_id: None, 
                };



                let resp_text = serde_json::to_string(&response_json).unwrap();

                let response = Response::from_string(resp_text)
                    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());

                request.respond(response).unwrap();
                continue;
            }

            // Handle other routes
            request
                .respond(Response::from_string("Invalid route"))
                .unwrap();
        }
    });
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            start_local_server();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
