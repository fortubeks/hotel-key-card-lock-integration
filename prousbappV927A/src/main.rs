use axum::{
    async_trait,
    extract::{FromRequestParts, State, Json},
    routing::{get, post},
    Router,
    http::{request::Parts, StatusCode, Method},
};
use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use std::{ffi::{CStr, CString}, os::raw::c_char, sync::Arc, env, net::SocketAddr};
use tokio::sync::{mpsc, oneshot};
use tower_http::cors::{CorsLayer, Any};

// --- DLL Type Definitions ---
type InitializeUSB = extern "system" fn(u8) -> i32;
type ReadCard = extern "system" fn(u8, *mut c_char) -> i32;
type GuestCard = extern "system" fn(u8, i32, u8, u8, u8, u8, *const i8, *const i8, *const i8, *mut i8) -> i32;
type CardErase = extern "system" fn(u8, i32, *mut c_char) -> i32;
type GetCardTypeByCardDataStr = extern "system" fn(*const c_char, *mut c_char) -> i32;
type GetGuestLockNoByCardDataStr = extern "system" fn(i32, *const c_char, *mut c_char) -> i32;
type GetGuestETimeByCardDataStr = extern "system" fn(i32, *const c_char, *mut c_char) -> i32;
type Buzzer = extern "system" fn(u8, i32) -> i32;

// --- Worker Command Types ---
pub enum DllCommand {
    Read { resp: oneshot::Sender<Result<CardData, String>> },
    Write { req: IssueRequest, resp: oneshot::Sender<Result<String, String>> },
    Erase { resp: oneshot::Sender<Result<String, String>> },
}

#[derive(Clone)]
pub struct CardData {
    pub hex: String,
    pub card_type: String,
    pub lock_no: String,
    pub expiry: String,
    pub hotel_id: i32,
    pub card_type_status: i32,
    pub lock_status: i32,
    pub expiry_status: i32,
}

#[derive(Deserialize)]
pub struct IssueRequest {
    pub hotel_id: i32,
    pub card_no: u8,
    pub dai: u8,
    pub unlock_deadbolt: u8,
    pub begin_date: String, 
    pub end_date: String,   
    pub lock_no: String,    
}



#[derive(Clone)]
pub struct AppState {
    pub api_token: String,
    pub tx: mpsc::Sender<DllCommand>,
}

pub struct AuthToken;

#[async_trait]
impl FromRequestParts<AppState> for AuthToken
where
    AppState: Send + Sync,
{
    type Rejection = (StatusCode, Json<ApiResponse>);

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let auth_header = parts.headers.get(axum::http::header::AUTHORIZATION)
            .ok_or((StatusCode::UNAUTHORIZED, Json(ApiResponse::error("Authorization header missing"))))?;

        let auth_str = auth_header.to_str()
            .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiResponse::error("Invalid Auth header"))))?;

        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            if token == state.api_token {
                return Ok(AuthToken);
            }
        }
        Err((StatusCode::UNAUTHORIZED, Json(ApiResponse::error("Invalid bearer token"))))
    }
}


#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub status: String,
    pub message: String,
}

impl ApiResponse {
    pub fn success(msg: &str) -> Self {
        Self { status: "success".to_string(), message: msg.to_string() }
    }
    pub fn error(msg: &str) -> Self {
        Self { status: "error".to_string(), message: msg.to_string() }
    }
}

#[derive(Debug, Serialize)]
pub struct CardResponse {
    pub status: String,
    pub message: String,
    pub card_snr: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GuestCardReadResponse {
    pub status: String,
    pub message: String,
    pub card_info: Option<GuestCardInfoDto>,
}

#[derive(Debug, Serialize)]
pub struct GuestCardInfoDto {
    pub card_snr: String,
    pub raw_card_data: String,
    pub card_type: String,
    pub lock_no: String,
    pub checkin_time: String,
    pub checkout_time: String,
    pub hotel_id: i32,
    pub card_type_status: i32,
    pub lock_status: i32,
    pub checkout_status: i32,
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<DllCommand>(32);
    
    // Load Token from config/auth
    let api_token = std::fs::read_to_string("./config/auth")
        .unwrap_or_else(|_| "default_secret".to_string())
        .trim().to_string();

    // --- BACKGROUND DLL WORKER ---
    std::thread::spawn(move || {
        let dll_path = env::current_exe().unwrap().parent().unwrap().join("proRFL.dll");
        let lib = unsafe { Library::new(dll_path).expect("proRFL.dll not found") };
        
        unsafe {
            let init: Symbol<InitializeUSB> = lib.get(b"initializeUSB").unwrap();
            init(1);
        }

        while let Some(cmd) = rx.blocking_recv() {
            match cmd {
                DllCommand::Read { resp } => {
                    let res = unsafe { worker_read(&lib) };
                    let _ = resp.send(res);
                }
                DllCommand::Write { req, resp } => {
                    let res = unsafe { worker_write(&lib, req) };
                    let _ = resp.send(res);
                }
                DllCommand::Erase { resp } => {
                    let res = unsafe { worker_erase(&lib) };
                    let _ = resp.send(res);
                }
            }
        }
    });

    let app_state = AppState { api_token, tx };

    let app = Router::new()
        .route("/card/read-guest", get(read_guest_handler))
        .route("/card/make-guest", post(make_guest_handler))
        .route("/card/cancel", post(cancel_handler))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 9001));
    println!("✓ RFID Encoder API running on http://{}", addr);
    println!("=========================\n");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- Worker Implementations (Internal Logic) ---

unsafe fn derive_id(hex: &str) -> i32 {
    if hex.len() < 14 { return 0; }
    let v1 = i32::from_str_radix(&hex[10..14], 16).unwrap_or(0) % 16384;
    let v2 = i32::from_str_radix(&hex[8..10], 16).unwrap_or(0) * 65536;
    v1 + v2
}

fn card_serial(card_data: &str) -> String {
    card_data.chars().skip(24).take(8).collect()
}

fn validate_issue_request(req: &IssueRequest) -> Result<(), String> {
    if req.hotel_id <= 0 {
        return Err("ProUSB Hotel/Company ID is required and must be greater than zero".into());
    }

    for (label, value) in [("begin_date", &req.begin_date), ("end_date", &req.end_date)] {
        if value.len() != 10 || !value.chars().all(|character| character.is_ascii_digit()) {
            return Err(format!("{} must contain exactly 10 digits in YYMMDDHHMM format", label));
        }
    }

    if req.lock_no.is_empty()
        || req.lock_no.len() > 8
        || !req.lock_no.chars().all(|character| character.is_ascii_alphanumeric())
    {
        return Err("ProUSB lock number must contain 1 to 8 letters or digits".into());
    }

    Ok(())
}

unsafe fn worker_read(lib: &Library) -> Result<CardData, String> {
    let read_card: Symbol<ReadCard> = lib.get(b"ReadCard").unwrap();
    let get_card_type: Symbol<GetCardTypeByCardDataStr> = lib.get(b"GetCardTypeByCardDataStr").unwrap();
    let get_lock: Symbol<GetGuestLockNoByCardDataStr> = lib.get(b"GetGuestLockNoByCardDataStr").unwrap();
    let get_time: Symbol<GetGuestETimeByCardDataStr> = lib.get(b"GetGuestETimeByCardDataStr").unwrap();

    let mut buf = [0 as c_char; 512];
    if read_card(1, buf.as_mut_ptr()) != 0 { return Err("No card detected".into()); }

    let hex = CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned();
    let hotel_id = derive_id(&hex);

    let mut l_buf = [0 as c_char; 32];
    let mut t_buf = [0 as c_char; 32];
    let mut type_buf = [0 as c_char; 8];
    let card_type_status = get_card_type(buf.as_ptr(), type_buf.as_mut_ptr());
    let lock_status = get_lock(hotel_id, buf.as_ptr(), l_buf.as_mut_ptr());
    let expiry_status = get_time(hotel_id, buf.as_ptr(), t_buf.as_mut_ptr());

    Ok(CardData {
        hex,
        card_type: CStr::from_ptr(type_buf.as_ptr()).to_string_lossy().into_owned(),
        lock_no: CStr::from_ptr(l_buf.as_ptr()).to_string_lossy().into_owned(),
        expiry: CStr::from_ptr(t_buf.as_ptr()).to_string_lossy().into_owned(),
        hotel_id,
        card_type_status,
        lock_status,
        expiry_status,
    })
}

unsafe fn worker_write(lib: &Library, req: IssueRequest) -> Result<String, String> {
    validate_issue_request(&req)?;

    let read_fn: Symbol<ReadCard> = lib.get(b"ReadCard").unwrap();
    let guest_fn: Symbol<GuestCard> = lib.get(b"GuestCard").unwrap();
    let get_card_type: Symbol<GetCardTypeByCardDataStr> = lib.get(b"GetCardTypeByCardDataStr").unwrap();
    let get_lock: Symbol<GetGuestLockNoByCardDataStr> = lib.get(b"GetGuestLockNoByCardDataStr").unwrap();
    let get_time: Symbol<GetGuestETimeByCardDataStr> = lib.get(b"GetGuestETimeByCardDataStr").unwrap();
    let buzz_fn: Symbol<Buzzer> = lib.get(b"Buzzer").unwrap();

    let mut buf = [0 as c_char; 512];
    if read_fn(1, buf.as_mut_ptr()) != 0 { return Err("Place card first".into()); }

    let raw_card_data = CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned();
    let derived_hotel_id = derive_id(&raw_card_data);
    let mut type_buf = [0 as c_char; 8];
    let type_status = get_card_type(buf.as_ptr(), type_buf.as_mut_ptr());
    if type_status != 0 {
        return Err(format!("Card type validation failed with SDK code {}. Card was not changed.", type_status));
    }

    let card_type = CStr::from_ptr(type_buf.as_ptr()).to_string_lossy().into_owned();
    match card_type.as_str() {
        "6" => {
            let mut existing_lock = [0 as c_char; 32];
            let validation_status = get_lock(req.hotel_id, buf.as_ptr(), existing_lock.as_mut_ptr());
            if validation_status != 0 {
                return Err(format!(
                    "Existing guest card does not belong to configured hotel {} (SDK code {}, derived ID {}). Card was not changed.",
                    req.hotel_id, validation_status, derived_hotel_id
                ));
            }
        }
        "F" => {}
        other => {
            return Err(format!(
                "Card type {} cannot be overwritten as a guest card. Use a blank card or an existing guest card.",
                if other.is_empty() { "unknown" } else { other }
            ));
        }
    }
    
    let b_date = CString::new(req.begin_date).unwrap();
    let e_date = CString::new(req.end_date).unwrap();
    let l_no = CString::new(req.lock_no).unwrap();
    let mut out = [0 as c_char; 512];

    let res = guest_fn(1, req.hotel_id, req.card_no, req.dai, req.unlock_deadbolt, 0,
                       b_date.as_ptr(), e_date.as_ptr(), l_no.as_ptr(), out.as_mut_ptr());

    if res != 0 {
        return Err(format!("Hardware Error: {}", res));
    }

    let mut verify_buf = [0 as c_char; 512];
    let verify_read_status = read_fn(1, verify_buf.as_mut_ptr());
    if verify_read_status != 0 {
        return Err(format!("Card was written but verification read failed with SDK code {}", verify_read_status));
    }

    let mut verified_lock_buf = [0 as c_char; 32];
    let mut verified_time_buf = [0 as c_char; 32];
    let lock_status = get_lock(req.hotel_id, verify_buf.as_ptr(), verified_lock_buf.as_mut_ptr());
    let time_status = get_time(req.hotel_id, verify_buf.as_ptr(), verified_time_buf.as_mut_ptr());
    if lock_status != 0 || time_status != 0 {
        return Err(format!(
            "Card was written but the configured hotel could not verify it (lock SDK code {}, checkout SDK code {})",
            lock_status, time_status
        ));
    }

    let verified_lock = CStr::from_ptr(verified_lock_buf.as_ptr()).to_string_lossy().into_owned();
    if verified_lock != req.lock_no {
        return Err(format!(
            "Card was written for lock number {}, but ProUSB decoded lock number {}. Check the room lock number configuration.",
            req.lock_no, verified_lock
        ));
    }

    buzz_fn(1, 20);
    Ok(format!("Success; card verified for hotel {} and lock {}", req.hotel_id, verified_lock))
}

// unsafe fn worker_erase(lib: &Library) -> Result<String, String> {
//     let read_fn: Symbol<ReadCard> = lib.get(b"ReadCard").unwrap();
//     let erase_fn: Symbol<CardErase> = lib.get(b"CardErase").unwrap();
//     let buzz_fn: Symbol<Buzzer> = lib.get(b"Buzzer").unwrap();

//     let mut buf = [0 as c_char; 512];
//     read_fn(1, buf.as_mut_ptr());
//     let hotel_id = derive_id(&CStr::from_ptr(buf.as_ptr()).to_string_lossy());

//     let mut out = [0 as c_char; 512];
//     if erase_fn(1, hotel_id, out.as_mut_ptr()) == 0 {
//         buzz_fn(1, 20); Ok("Card Cancelled".into())
//     } else { Err("Hardware rejection".into()) }
// }

unsafe fn worker_erase(lib: &Library) -> Result<String, String> {
    let read_fn: Symbol<ReadCard> = lib.get(b"ReadCard").unwrap();
    let erase_fn: Symbol<CardErase> = lib.get(b"CardErase").unwrap();
    let buzz_fn: Symbol<Buzzer> = lib.get(b"Buzzer").unwrap();

    let mut buf = [0 as c_char; 512];

    if read_fn(1, buf.as_mut_ptr()) != 0 {
        return Err("No card detected".into());
    }

    let hex = CStr::from_ptr(buf.as_ptr()).to_string_lossy();
    let hotel_id = derive_id(&hex);

    let res = erase_fn(1, hotel_id, buf.as_mut_ptr());

    if res == 0 {
        buzz_fn(1, 20);
        Ok("Card Cancelled".into())
    } else {
        Err(format!("Erase failed, code {}", res))
    }
}


// --- API Handlers ---

async fn read_guest_handler(State(state): State<AppState>, _auth: AuthToken) -> Json<GuestCardReadResponse> {
    println!("/card/read-guest - Reading guest card");

    let (tx, rx) = oneshot::channel();
    let _ = state.tx.send(DllCommand::Read { resp: tx }).await;
    
    match rx.await {
        Ok(Ok(data)) => Json(GuestCardReadResponse {
            status: "success".into(),
            message: "Read OK".into(),
            card_info: Some(GuestCardInfoDto {
                card_snr: card_serial(&data.hex),
                raw_card_data: data.hex,
                card_type: data.card_type,
                lock_no: data.lock_no,
                checkin_time: "N/A".into(),
                checkout_time: data.expiry,
                hotel_id: data.hotel_id,
                card_type_status: data.card_type_status,
                lock_status: data.lock_status,
                checkout_status: data.expiry_status,
            }),
        }),
        _ => Json(GuestCardReadResponse { status: "error".into(), message: "Read failed".into(), card_info: None }),
    }
}

async fn make_guest_handler(State(state): State<AppState>, _auth: AuthToken, Json(req): Json<IssueRequest>) -> Json<CardResponse> {
    println!("/card/make-guest - Creating guest card");

    let (tx, rx) = oneshot::channel();
    let _ = state.tx.send(DllCommand::Write { req, resp: tx }).await;
    
    match rx.await {
        Ok(Ok(m)) => Json(CardResponse { status: "success".into(), message: m, card_snr: None }),
        Ok(Err(e)) => Json(CardResponse { status: "error".into(), message: e, card_snr: None }),
        _ => Json(CardResponse { status: "error".into(), message: "Worker timeout".into(), card_snr: None }),
    }
}

async fn cancel_handler(State(state): State<AppState>, _auth: AuthToken) -> Json<CardResponse> {
    println!("/card/cancel - Cancelling card");
    
    let (tx, rx) = oneshot::channel();
    let _ = state.tx.send(DllCommand::Erase { resp: tx }).await;
    
    match rx.await {
        Ok(Ok(m)) => Json(CardResponse { status: "success".into(), message: m, card_snr: None }),
        _ => Json(CardResponse { status: "error".into(), message: "Erase failed".into(), card_snr: None }),
    }
}
