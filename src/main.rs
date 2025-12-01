use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use imagepreview::grid::ImageService;
use std::sync::Arc;
use tower_http::trace::{TraceLayer, DefaultMakeSpan, DefaultOnResponse};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn hello_world() -> &'static str {
    "Hello, World!"
}

async fn preview_handler(
    State(service): State<Arc<ImageService>>,
    Path(base64_urls): Path<String>,
) -> impl IntoResponse {
    let base64_urls = base64_urls.trim_start_matches('/');

    match service.process_base64_urls(base64_urls).await {
        Ok(image) => {
            let mut buf = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut buf);

            // Convert to DynamicImage for JPEG encoding
            let dynamic_image = image::DynamicImage::ImageRgba8(image);

            if let Err(e) = dynamic_image.write_to(&mut cursor, image::ImageFormat::Jpeg) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "text/plain")],
                    format!("Failed to encode image: {}", e).into_bytes(),
                );
            }

            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "image/jpeg")],
                buf,
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "text/plain")],
            format!("Error: {}", e).into_bytes(),
        ),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "imagepreview=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let image_service = Arc::new(ImageService::new());

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/preview/*base64", get(preview_handler))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(image_service);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("Server running on http://localhost:3000");
    println!();
    println!("Usage:");
    println!("  GET /preview/{{base64-encoded-urls}}");
    println!();
    println!("Example:");
    println!("  URLS='https://example.com/1.jpg,https://example.com/2.png'");
    println!("  BASE64=$(echo -n \"$URLS\" | base64)");
    println!("  curl http://localhost:3000/preview/$BASE64 > grid.jpg");

    axum::serve(listener, app)
        .await
        .unwrap();
}
