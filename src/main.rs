use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_session::{Session, SessionMiddleware};
use actix_web::{App, HttpResponse, HttpServer, Responder, Result, cookie::Key, web};
use futures_util::StreamExt as _;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};


// Internal upload directory (relative to project)
static UPLOAD_DIR: &str = "C:\\Users\\ANKIT\Desktop\\";
static USERNAME: &str = "admin";
static PASSWORD: &str = "admin";

#[derive(Serialize)]
struct FileEntry {
    name: String,
    is_dir: bool,
    size: u64,
}

fn is_logged_in(session: &Session) -> bool {
    session
        .get::<bool>("logged_in")
        .unwrap_or(Some(false))
        .unwrap_or(false)
}

async fn login_form() -> impl Responder {
    let html = r#"
    <!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Ankit Server Login</title>
    <link rel="icon" type="image/png" href="https://www.pngplay.com/wp-content/uploads/7/Cloud-Server-Icon-Transparent-Images.png">
    <script src="https://cdn.tailwindcss.com"></script>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head>
<body class="flex items-center justify-center min-h-screen bg-gray-100">
    <div class="w-full max-w-md bg-white rounded-lg shadow-lg p-6">
        <div class="text-center mb-6">
            <h2 class="text-2xl font-bold text-gray-700"><i class="fas fa-cloud"></i> Ankit Server Login</h2>
            <p class="text-gray-500 text-sm">Restricted Access - Authorized Personnel Only</p>
        </div>
        
        <form class="space-y-4" method="post" action="/login">
            <div>
                <label class="block text-gray-600 mb-1"><i class="fas fa-user-cog"></i> Admin ID</label>
                <input type="text" name="username" class="w-full border rounded px-3 py-2" placeholder="Enter admin username" required>
            </div>
            <div>
                <label class="block text-gray-600 mb-1"><i class="fas fa-key"></i> Security Key</label>
                <input type="password" name="password" class="w-full border rounded px-3 py-2" placeholder="Enter password" required>
            </div>
            <div class="text-yellow-600 text-sm"><i class="fas fa-exclamation-circle"></i> Ensure you're on a secure network before logging in</div>
            <button type="submit" class="w-full bg-blue-600 hover:bg-blue-700 text-white py-2 rounded"><i class="fas fa-sign-in-alt"></i> Login</button>
        </form>
    </div>
</body>
</html>
    "#;
    HttpResponse::Ok().content_type("text/html").body(html)
}

#[derive(serde::Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

// async fn login(session: Session, form: web::Form<LoginForm>) -> impl Responder {
//     if form.username == USERNAME && form.password == PASSWORD {
//         session.insert("logged_in", true).unwrap();
//         HttpResponse::SeeOther()
//             .append_header(("Location", "/"))
//             .finish()
//     } else {
//         HttpResponse::Unauthorized().body("Invalid username or password")
//     }
// }

async fn login(form: web::Form<LoginForm>, session: Session) -> Result<HttpResponse> {
    // check if locked
    if let Some(lock_until) = session.get::<u64>("lock_until")? {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if now < lock_until {
            let remaining = (lock_until - now) / 60; // in minutes
            return Ok(HttpResponse::Ok().body(format!("Too many attempts. Try again in {} minutes", remaining)));
        }
    }

    if form.username == USERNAME && form.password == PASSWORD {
         session.insert("logged_in", true).unwrap();
        session.remove("failed_attempts");
        session.remove("lock_until");
        return Ok(HttpResponse::SeeOther()
            .append_header(("Location", "/"))
            .finish());
    } else {
        // wrong password → increment failed attempts
        let mut attempts = session.get::<i32>("failed_attempts")?.unwrap_or(0);
        attempts += 1;
        session.insert("failed_attempts", attempts)?;

        if attempts >= 3 {
            let lock_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 30 * 60; // 30 minutes
            session.insert("lock_until", lock_time)?;
            return Ok(HttpResponse::Ok().body("Too many attempts. Account locked for 30 minutes."));
        }

        return Ok(HttpResponse::Unauthorized().body(format!(
            "Invalid credentials. {} attempts remaining.",
            3 - attempts
        )));
    }
}

async fn logout(session: Session) -> impl Responder {
    session.purge();
    HttpResponse::SeeOther()
        .append_header(("Location", "/login"))
        .finish()
}

async fn index(
    session: Session,
    web::Query(params): web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    if !is_logged_in(&session) {
        return HttpResponse::SeeOther()
            .append_header(("Location", "/login"))
            .finish();
    }

    let rel_path = params
        .get("path")
        .map(|s| s.trim_start_matches(&['/', '\\'][..]))
        .unwrap_or("");
    let full_path = Path::new(UPLOAD_DIR).join(rel_path);

    match read_dir_entries(&full_path) {
        Ok(entries) => {
            let html = render_index(&entries, rel_path);
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(html)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}

fn read_dir_entries<P: AsRef<Path>>(p: P) -> std::io::Result<Vec<FileEntry>> {
    let mut res = Vec::new();
    for entry in fs::read_dir(p)? {
        let e = entry?;
        let meta = e.metadata()?;
        let name = e.file_name().into_string().unwrap_or_default();
        res.push(FileEntry {
            name,
            is_dir: meta.is_dir(),
            size: meta.len(),
        });
    }
    res.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(res)
}

fn render_index(entries: &[FileEntry], rel_path: &str) -> String {
    let mut rows = String::new();
    for f in entries {
        let encoded = utf8_percent_encode(&f.name, NON_ALPHANUMERIC).to_string();
        let preview_link = if is_previewable(&f.name) {
            format!(
                "<a href=\"/preview/{encoded}?path={rel_path}\" target=\"_blank\" class=\"text-green-600 hover:underline\"><i class=\"fas fa-eye\"></i> Preview</a>",
                encoded = encoded,
                rel_path = rel_path
            )
        } else {
            String::new()
        };

        let size_text = if f.is_dir {
            "—".to_string()
        } else {
            format!("{} B", f.size)
        };

        let actions = if f.is_dir {
            format!(
                "<a href=\"/?path={rel_path}/{encoded}\" class=\"text-blue-600 hover:underline\">📂 Open</a> | \
                 <a href=\"/delete/{encoded}?path={rel_path}\" class=\"text-red-600 hover:underline\"><i class=\"fas fa-trash\"></i> Delete</a>",
                rel_path = rel_path,
                encoded = encoded
            )
        } else {
            format!(
                "<a href=\"/download/{encoded}?path={rel_path}\" class=\"text-blue-600 hover:underline\"><i class=\"fas fa-download\"></i> Download</a> | \
                 {preview} | \
                 <a href=\"/delete/{encoded}?path={rel_path}\" class=\"text-red-600 hover:underline\"><i class=\"fas fa-trash\"></i> Delete</a>",
                rel_path = rel_path,
                encoded = encoded,
                preview = preview_link
            )
        };

        rows.push_str(&format!(
            r#"<div class="border rounded-lg p-4 bg-white shadow">
                <div class="flex justify-between items-center">
                    <span class="font-semibold">{}</span>
                    <span class="text-gray-500 text-sm">{}</span>
                </div>
                <div class="text-sm text-gray-600">{}</div>
                <div class="mt-2 space-x-2">{}</div>
            </div>"#,
            escape_html(&f.name),
            size_text,
            if f.is_dir { "Dir" } else { "File" },
            actions
        ));
    }

    let back_button = if rel_path.is_empty() {
        "".to_string()
    } else {
        let parent = Path::new(rel_path)
            .parent()
            .map(|p| p.to_str().unwrap_or(""))
            .unwrap_or("");
        format!(
            "<a href=\"/?path={}\" class=\"bg-gray-200 px-3 py-1 rounded hover:bg-gray-300\"><i class=\"fas fa-arrow-left\"></i> Back</a>",
            parent
        )
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Ankit Server File Manager</title>
    <link rel="icon" type="image/png" href="https://www.pngplay.com/wp-content/uploads/7/Cloud-Server-Icon-Transparent-Images.png">
    <script src="https://cdn.tailwindcss.com"></script>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head>
<body class="bg-gray-100 min-h-screen">
    <div class="container mx-auto p-6">
        <header class="flex justify-between items-center mb-6">
            <h3 class="text-xl font-bold"><i class="fas fa-folder"></i>~$ {path}</h3><br/>
            <div class="space-x-2">
                {back_button}
                <a href="/logout" class="bg-red-500 text-white px-3 py-1 rounded hover:bg-red-600"><i class="fas fa-sign-out-alt"></i> Logout</a>
            </div>
        </header>

        <section class="mb-6">
            <form class="flex space-x-2" action="/upload?path={rel_path}" method="post" enctype="multipart/form-data">
                <input type="file" name="file" class="flex-1 border rounded px-3 py-2" required>
                <button type="submit" class="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700"><i class="fas fa-cloud-upload-alt"></i> Upload</button>
            </form>
        </section>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {rows}
        </div>

        <footer class="mt-10 text-center text-gray-500 text-sm">
            © 2025-30 File Manager | Built with <i class="fas fa-heart text-red-500"></i>
        </footer>
    </div>
</body>
</html>"#,
        path = escape_html(if rel_path.is_empty() {
            UPLOAD_DIR
        } else {
            rel_path
        }),
        rows = rows,
        back_button = back_button,
        rel_path = rel_path
    )
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn is_previewable(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".mp4")
        || lower.ends_with(".mp3")
        || lower.ends_with(".txt")
        || lower.ends_with(".html")
}

async fn upload(
    session: Session,
    mut payload: Multipart,
    web::Query(params): web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse> {
    if !is_logged_in(&session) {
        return Ok(HttpResponse::Unauthorized().finish());
    }

    let rel_path = params.get("path").map(|s| s.as_str()).unwrap_or("");
    let upload_path = Path::new(UPLOAD_DIR).join(rel_path);
    fs::create_dir_all(&upload_path)?;

    while let Some(field) = payload.next().await {
        let mut field = field?;
        let filename = field
            .content_disposition()
            .get_filename()
            .unwrap_or("upload.tmp")
            .to_string();
        let filepath = upload_path.join(&filename);
        let mut f = web::block(|| std::fs::File::create(filepath)).await??;
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            f = web::block(move || {
                let mut f = f;
                f.write_all(&data)?;
                Ok::<_, std::io::Error>(f)
            })
            .await??;
        }
    }
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/?path={}", rel_path)))
        .finish())
}

async fn download(
    session: Session,
    path: web::Path<String>,
    web::Query(params): web::Query<std::collections::HashMap<String, String>>,
) -> Result<NamedFile> {
    if !is_logged_in(&session) {
        return Err(actix_web::error::ErrorUnauthorized("Login required"));
    }

    let fname = path.into_inner();
    let decoded = percent_encoding::percent_decode_str(&fname).decode_utf8_lossy();
    let rel_path = params.get("path").map(|s| s.as_str()).unwrap_or("");
    let full_path = Path::new(UPLOAD_DIR).join(rel_path).join(&*decoded);

    if !full_path.exists() {
        return Err(actix_web::error::ErrorNotFound("File not found"));
    }
    Ok(NamedFile::open(full_path)?)
}

async fn preview(
    session: Session,
    path: web::Path<String>,
    web::Query(params): web::Query<std::collections::HashMap<String, String>>,
) -> Result<NamedFile> {
    if !is_logged_in(&session) {
        return Err(actix_web::error::ErrorUnauthorized("Login required"));
    }
    let fname = path.into_inner();
    let decoded = percent_encoding::percent_decode_str(&fname).decode_utf8_lossy();
    let rel_path = params.get("path").map(|s| s.as_str()).unwrap_or("");
    let p = Path::new(UPLOAD_DIR).join(rel_path).join(&*decoded);
    Ok(NamedFile::open(p)?)
}

async fn delete(
    session: Session,
    path: web::Path<String>,
    web::Query(params): web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse> {
    if !is_logged_in(&session) {
        return Ok(HttpResponse::Unauthorized().finish());
    }
    let fname = path.into_inner();
    let decoded = percent_encoding::percent_decode_str(&fname).decode_utf8_lossy();
    let rel_path = params.get("path").map(|s| s.as_str()).unwrap_or("");
    let p = Path::new(UPLOAD_DIR).join(rel_path).join(&*decoded);
    if p.exists() {
        if p.is_dir() {
            let _ = fs::remove_dir_all(&p);
        } else {
            let _ = fs::remove_file(&p);
        }
    }
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/?path={}", rel_path)))
        .finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    fs::create_dir_all(UPLOAD_DIR)?;

    let secret_key = Key::generate();

    println!("Server running at http://0.0.0.0:3076");

    HttpServer::new(move || {
        App::new()
            .wrap(SessionMiddleware::new(
                actix_session::storage::CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/logout", web::get().to(logout))
            .route("/", web::get().to(index))
            .route("/upload", web::post().to(upload))
            .route("/download/{filename}", web::get().to(download))
            .route("/preview/{filename}", web::get().to(preview))
            .route("/delete/{filename}", web::get().to(delete))
    })
    .bind(("127.0.0.1", 8000))? // You can change this Port For Runing 
    .run()
    .await
}

