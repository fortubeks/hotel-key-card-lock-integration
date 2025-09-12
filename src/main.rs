use axum::{
    async_trait,
    extract::{FromRequestParts, State, Query},
    http::{request::Parts, StatusCode, HeaderMap, Method},
    Json, Router,
    routing::{post, get},
};
use tokio::net::TcpListener;
use chrono::{Local, Datelike, Timelike, NaiveDateTime};
use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::Arc,
    path::Path,
    fs::read_to_string,
    str,
};
use hex;
use tower_http::cors::{CorsLayer, Any};

//api token file location
const API_TOKEN_FILE: &str = "./config/auth";

#[derive(Clone)]
struct AppState {
    lib: Arc<Library>,
    api_token: String,
}

pub struct AuthToken;

#[async_trait]
impl FromRequestParts<AppState> for AuthToken
where
    AppState: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;

        let auth_header = headers.get(hyper::header::AUTHORIZATION)
            .ok_or((StatusCode::UNAUTHORIZED, "Authorization header missing"))?;

        // Convert the header value to a string
        let auth_str = auth_header.to_str()
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Authorization header"))?;

        //extract and validate token
        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            if token == state.api_token {
                Ok(AuthToken)
            } else {
                Err((StatusCode::UNAUTHORIZED, "Invalid bearer token"))
            }
        } else {
            Err((StatusCode::UNAUTHORIZED, "Bearer token not found in Authorization header"))
        }
    }
}

#[derive(Debug, Deserialize)]
struct GuestCardRequest {
    dls_co_id: i32,
    card_no: u8,
    dai: u8,
    unlock_deadbolt: u8,
    public_door: u8,
    begin: String,
    end: String,
    lock_no: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct WriteCardResponse {
    status: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GuestCardResponse {
    status: String,
    message: String,
    card_hex: Option<String>,
    card_id: Option<String>,
    lock_no: Option<String>,
    expiry: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EraseCardRequest {
    dls_co_id: i32,
}

#[derive(Debug, Deserialize)]
struct ReadCardParams {
    dls_co_id: Option<i32>,
}

// New struct for the extract-coid response
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ExtractCoidResponse {
    status: String,
    message: String,
    coid: Option<u32>,
}

#[derive(Serialize)]
struct TestResponse {
    status: String,
    message: String,
}

#[tokio::main]
async fn main() {
    // Load the DLL library
    let lib = Arc::new(unsafe {
        Library::new("proRFL.dll").expect("Failed to load DLL: proRFL.dll. Make sure it's in the same directory.")
    });

    // Load the API token from file
    let api_token = load_api_token()
        .expect(&format!("Failed to load API token from {}", API_TOKEN_FILE));
    //println!("API Token loaded from: {}", API_TOKEN_FILE);

    // Create the shared application state
    let app_state = AppState {
        lib: lib.clone(),
        api_token,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        // Routes now require AuthToken extractor
        .route("/write-card", post(write_card_handler))
        .route("/read-card", get(read_card_handler))
        .route("/erase-card", post(erase_card_handler))
        .route("/get-coid", get(extract_coid_handler))
        .route("/test", get(test_endpoint_handler)) 
        .with_state(app_state) // Pass the AppState to the router
        .layer(cors); 

    let addr: SocketAddr = "127.0.0.1:9001".parse().unwrap();
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    println!("Listening on http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}

// Function to load the API token from a file
fn load_api_token() -> Result<String, Box<dyn std::error::Error>> {
    let path = Path::new(API_TOKEN_FILE);
    if !path.exists() {
        return Err(format!("Error: API token file not found at {:?}. Please create it and add your token.", path.canonicalize()?).into());
    }
    let token = read_to_string(path)?.trim().to_string();
    if token.is_empty() {
        return Err(format!("Error: API token file at {:?} is empty. Please add your token.", path.canonicalize()?).into());
    }
    Ok(token)
}

async fn test_endpoint_handler() -> Json<TestResponse> {
    Json(TestResponse {
        status: "success".to_string(),
        message: "API is running!".to_string(),
    })
}

// Handlers now include the AuthToken extractor
async fn write_card_handler(
    _auth: AuthToken, // Requires valid authentication
    State(state): State<AppState>, // Use State to get the entire AppState
    Json(payload): Json<GuestCardRequest>,
) -> Json<WriteCardResponse> {
    let result = unsafe { write_guest_card(&state.lib, &payload) };
    println!("/write-card ...");

    match result {
        Ok((_hex, _card_id)) => {
            Json(WriteCardResponse {
                status: "success".to_string(),
                message: "Card written successfully".to_string(),
            })
        }
        Err((_code, msg)) => Json(WriteCardResponse {
            status: "error".to_string(),
            message: msg,
        }),
    }
}

async fn read_card_handler(
    _auth: AuthToken, // Requires valid authentication
    State(state): State<AppState>, // Use State to get the entire AppState
    Query(params): Query<ReadCardParams>,
) -> Json<GuestCardResponse> {
    println!("/read-card ...");
    let dls_co_id = params.dls_co_id.unwrap_or(12918835);
    let read_result = unsafe { read_card_info(&state.lib, dls_co_id) };
    Json(read_result)
}

async fn erase_card_handler(
    _auth: AuthToken, // Requires valid authentication
    State(state): State<AppState>, // Use State to get the entire AppState
    Json(payload): Json<EraseCardRequest>,
) -> Json<GuestCardResponse> {
    println!("/erase-card ...");

    let result = unsafe { erase_card(&state.lib, payload.dls_co_id) };
    Json(result)
}

// Modified handler for COID extraction to directly read card data
async fn extract_coid_handler(
    _auth: AuthToken,
    State(state): State<AppState>, // Needs AppState to access the library
) -> Json<ExtractCoidResponse> {
    println!("/get-coid ...");
    let usb_type = 1;
    let dls_co_id = 12918835; // Default dls_co_id for buzzer

    let response = unsafe {
        // Load symbols directly within the unsafe block where they are used
        let initialize_usb: Symbol<unsafe extern "stdcall" fn(u8) -> i32> =
            state.lib.get(b"initializeUSB").unwrap();
        let read_card: Symbol<unsafe extern "stdcall" fn(u8, *mut u8) -> i32> =
            state.lib.get(b"ReadCard").unwrap();
        let sound_buzzer: Symbol<unsafe extern "stdcall" fn(u8, i32) -> i32> =
            state.lib.get(b"Buzzer").unwrap();

        if initialize_usb(usb_type) != 0 {
            return Json(ExtractCoidResponse {
                status: "error".to_string(),
                message: "USB initialization failed".to_string(),
                coid: None,
            });
        }

        let mut card_data = [0u8; 128]; // Buffer for raw card data
        let read_result_code = read_card(usb_type, card_data.as_mut_ptr());

        if read_result_code != 0 {
            return Json(ExtractCoidResponse {
                status: "error".to_string(),
                message: format!("Card read failed with code {}", read_result_code),
                coid: None,
            });
        }

        // Card read successful, now try to extract COID
        // Ensure enough bytes for indices 8, 10, 11 for extract_coid
        if card_data.len() < 12 {
            Json(ExtractCoidResponse {
                status: "error".to_string(),
                message: "Raw card data too short for COID extraction".to_string(),
                coid: None,
            })
        } else {
            let coid = extract_coid(&card_data);
            sound_buzzer(usb_type, dls_co_id); // Buzzer on successful read

            Json(ExtractCoidResponse {
                status: "success".to_string(),
                message: "COID extracted successfully".to_string(),
                coid: Some(coid),
            })
        }
    }; // End of unsafe block and `response` assignment
    response // Return the Json response
}


unsafe fn write_guest_card(lib: &Library, req: &GuestCardRequest) -> Result<(String, String), (i32, String)> {
    // Expected input format: "yymmddhhmm" (e.g., "2508010930" for 2025-08-01 09:30)
    let begin_date = NaiveDateTime::parse_from_str(&req.begin, "%y%m%d%H%M")
    .map_err(|_| (400, "Invalid start date format. Use YYMMDDHHMM".into()))?;

    let end_date = NaiveDateTime::parse_from_str(&req.end, "%y%m%d%H%M")
        .map_err(|_| (400, "Invalid end date format. Use YYMMDDHHMM".into()))?;

    let now = Local::now().naive_local();

    if begin_date < now || end_date < now {
        return Err((401, "Start and end dates must not be in the past.".into()));
    }

    if begin_date > end_date {
        return Err((402, "Start date cannot be after end date.".into()));
    }
    
    let initialize_usb: Symbol<unsafe extern "stdcall" fn(u8) -> i32> =
        lib.get(b"initializeUSB").unwrap();
    let sound_buzzer: Symbol<unsafe extern "stdcall" fn(u8, i32) -> i32> =
        lib.get(b"Buzzer").unwrap();
    let read_card: Symbol<unsafe extern "stdcall" fn(u8, *mut u8) -> i32> =
        lib.get(b"ReadCard").unwrap();
    let card_erase: Symbol<unsafe extern "stdcall" fn(u8, i32, *mut u8) -> i32> =
        lib.get(b"CardErase").unwrap();
    let guest_card: Symbol<
        unsafe extern "stdcall" fn(
            u8,
            i32,
            u8,
            u8,
            u8,
            u8,
            *const u8,
            *const u8,
            *const u8,
            *mut u8,
        ) -> i32,
    > = lib.get(b"GuestCard").unwrap();

    let usb_type = 1;

    if initialize_usb(usb_type) != 0 {
        return Err((100, "USB initialization failed".into()));
    }

    // Try to read the card first
    let mut card_buf = [0u8; 512];
    let read_result = read_card(usb_type, card_buf.as_mut_ptr());
    if read_result != 0 {
        return Err((101, "Failed to read card before erase".into()));
    }

    // Try to erase the card
    // let erase_result = card_erase(usb_type, req.dls_co_id, card_buf.as_mut_ptr());
    // if erase_result != 0 {
    //     return Err((102, format!("Failed to erase card: error code {}", erase_result)));
    // }

    // Prepare and write
    let bdate = fixed_buffer(&req.begin, 10);
    let edate = fixed_buffer(&req.end, 10);
    let lock_no = fixed_buffer(&req.lock_no, 8);
    let mut out_buf = [0u8; 256];

    let write_result = guest_card(
        usb_type,
        req.dls_co_id,
        req.card_no,
        req.dai,
        req.unlock_deadbolt,
        req.public_door,
        bdate.as_ptr(),
        edate.as_ptr(),
        lock_no.as_ptr(),
        out_buf.as_mut_ptr(),
    );

    if write_result == 0 {
        sound_buzzer(usb_type, req.dls_co_id);

        let hex = std::str::from_utf8(&out_buf)
            .unwrap_or("")
            .trim_matches(char::from(0))
            .to_string();

        let card_id = if hex.len() >= 32 {
            hex[24..32].to_string()
        } else {
            "".to_string()
        };

        Ok((hex, card_id))
    } else {
        Err((103, format!("GuestCard failed with code {}", write_result)))
    }
}

fn fixed_buffer(s: &str, size: usize) -> [u8; 16] {
    let mut buf = [0u8; 16];
    for (i, b) in s.bytes().take(size).enumerate() {
        buf[i] = b;
    }
    buf
}

// fn parse_expiry(raw: &str) -> Option<String> {
//     if raw.len() != 10 {
//         return None;
//     }

//     let formatted = format!(
//         "20{}-{}-{} {}:{}",
//         &raw[0..2], &raw[2..4], &raw[4..6], &raw[6..8], &raw[8..10]
//     );

//     NaiveDateTime::parse_from_str(&formatted, "%Y-%m-%d %H:%M")
//         .ok()
//         .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
// }


unsafe fn read_card_info(lib: &Library, dls_co_id: i32) -> GuestCardResponse {
    let initialize_usb: Symbol<unsafe extern "stdcall" fn(u8) -> i32> =
        lib.get(b"initializeUSB").unwrap();
    let sound_buzzer: Symbol<unsafe extern "stdcall" fn(u8, i32) -> i32> =
        lib.get(b"Buzzer").unwrap();
    let read_card: Symbol<unsafe extern "stdcall" fn(u8, *mut u8) -> i32> =
        lib.get(b"ReadCard").unwrap();
    let get_lock_no: Symbol<unsafe extern "stdcall" fn(i32, *const u8, *mut u8) -> i32> =
        lib.get(b"GetGuestLockNoByCardDataStr").unwrap();
    let get_expiry: Symbol<unsafe extern "stdcall" fn(i32, *const u8, *mut u8) -> i32> =
        lib.get(b"GetGuestETimeByCardDataStr").unwrap();

    let usb_type = 1;

    if initialize_usb(usb_type) != 0 {
        return GuestCardResponse {
            status: "error".into(),
            message: "USB initialization failed".into(),
            card_hex: None,
            card_id: None,
            lock_no: None,
            expiry: None,
        };
    }

    let mut card_data = [0u8; 128];
    let read_result = read_card(usb_type, card_data.as_mut_ptr());
    if read_result != 0 {
        return GuestCardResponse {
            status: "error".into(),
            message: format!("Card read failed with code {}", read_result),
            card_hex: None,
            card_id: None,
            lock_no: None,
            expiry: None,
        };
    }

    if let Ok(data_str) = str::from_utf8(&card_data) {
        let trimmed = data_str.trim_matches(char::from(0));
        if trimmed.starts_with("551501") {
            let card_id = if trimmed.len() >= 32 {
                Some(trimmed[24..32].to_string())
            } else {
                None
            };

            let mut lock_buf = [0u8; 16];
            get_lock_no(dls_co_id, card_data.as_ptr(), lock_buf.as_mut_ptr());
            let lock_str = str::from_utf8(&lock_buf).unwrap_or("").trim_matches(char::from(0)).to_string();

            let mut expiry_buf = [0u8; 16];
            get_expiry(dls_co_id, card_data.as_ptr(), expiry_buf.as_mut_ptr());
            let expiry_str = str::from_utf8(&expiry_buf).unwrap_or("").trim_matches(char::from(0));
            
            let formatted_expiry = parse_expiry(expiry_str);

            sound_buzzer(usb_type, dls_co_id);

            return GuestCardResponse {
                status: "success".into(),
                message: "Card read successfully".into(),
                card_hex: Some(trimmed.to_string()),
                card_id,
                lock_no: Some(lock_str),
                expiry: formatted_expiry,
            };
        }
    }

    GuestCardResponse {
        status: "error".into(),
        message: "Invalid or unreadable card data".into(),
        card_hex: None,
        card_id: None,
        lock_no: None,
        expiry: None,
    }
}

unsafe fn erase_card(lib: &Library, dls_co_id: i32) -> GuestCardResponse {
    let initialize_usb: Symbol<unsafe extern "stdcall" fn(u8) -> i32> =
        lib.get(b"initializeUSB").unwrap();
    let sound_buzzer: Symbol<unsafe extern "stdcall" fn(u8, i32) -> i32> =
        lib.get(b"Buzzer").unwrap();
    let read_card: Symbol<unsafe extern "stdcall" fn(u8, *mut u8) -> i32> =
        lib.get(b"ReadCard").unwrap();
    let card_erase: Symbol<unsafe extern "stdcall" fn(u8, i32, *mut u8) -> i32> =
        lib.get(b"CardErase").unwrap();

    let usb_type = 1;
    if initialize_usb(usb_type) != 0 {
        return GuestCardResponse {
            status: "error".into(),
            message: "USB initialization failed".into(),
            card_hex: None,
            card_id: None,
            lock_no: None,
            expiry: None,
        };
    }

    let mut card_data = [0u8; 128];
    if read_card(usb_type, card_data.as_mut_ptr()) != 0 {
        return GuestCardResponse {
            status: "error".into(),
            message: "Failed to read card before erasing".into(),
            card_hex: None,
            card_id: None,
            lock_no: None,
            expiry: None,
        };
    }

    if card_erase(usb_type, dls_co_id, card_data.as_mut_ptr()) != 0 {
        sound_buzzer(usb_type, dls_co_id);

        return GuestCardResponse {
            status: "error".into(),
            message: "Card erase failed".into(),
            card_hex: None,
            card_id: None,
            lock_no: None,
            expiry: None,
        };
    }

    GuestCardResponse {
        status: "success".into(),
        message: "Card erased successfully".into(),
        card_hex: None,
        card_id: None,
        lock_no: None,
        expiry: None,
    }
}

fn extract_coid(card_data: &[u8]) -> u32 {
    // Convert card bytes to ASCII string (not hex!)
    let card_str = match std::str::from_utf8(card_data) {
        Ok(s) => s.trim_matches(char::from(0)),
        Err(_) => return 0,
    };

    // Ensure card is long enough
    if card_str.len() < 14 {
        return 0;
    }

    // VB: Mid(bufCard, 11, 4) → characters 10..14 (0-based)
    let s1 = &card_str[10..14];
    let low = u32::from_str_radix(s1, 16).unwrap_or(0) % 16384;

    // VB: Mid(bufCard, 9, 2) → characters 8..10
    let s2 = &card_str[8..10];
    let high = u32::from_str_radix(s2, 16).unwrap_or(0);

    low + (high * 65536)
}


fn corrected_year(dll_year: i32) -> i32 {
    // let current_year = Local::now().year();
    // let base_year = current_year - 16;
    // let mut corrected = dll_year;
    // while corrected < current_year {
    //     corrected += 16;
    // }
    // corrected

    
    let current_year = Local::now().year();
    
    let base_year = 2009;
    let last_two_digits = (current_year-base_year) % 100;
    let cycle = last_two_digits / 16;

    let mut corrected_year = dll_year + (cycle * 16);

    if corrected_year < current_year {
        corrected_year += 16;
    }

    corrected_year

    // let now = Local::now();
    // let mut current_year = now.year();

    // // If today is 31st December, assume we're preparing for the next year
    // if now.month() == 12 && now.day() == 31 {
    //     current_year += 1;
    // }

    // let last_two_digits = current_year % 100;
    // let cycle = last_two_digits / 16;

    // dll_year + (cycle * 16)

}

fn parse_expiry(raw: &str) -> Option<String> {
    if raw.len() != 10 {
        return None;
    }

    let yy = raw[0..2].parse::<i32>().ok()?;
    let year = corrected_year(2000 + yy);
    let formatted = format!(
        "{}-{}-{} {}:{}",
        year,
        &raw[2..4],
        &raw[4..6],
        &raw[6..8],
        &raw[8..10]
    );

    NaiveDateTime::parse_from_str(&formatted, "%Y-%m-%d %H:%M")
        .ok()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
}
