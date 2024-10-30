pub struct RequestOptions {
    pub batch_size: usize,
    pub max_concurrency: usize,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            batch_size: 10,
            max_concurrency: 5,
        }
    }
}

pub struct PdfOptions {
    pub max_width: f64,
    pub line_height: f64,
    pub paragraph_spacing: f64,
    pub min_y_pos: f64,
    pub max_y_pos: f64,
    pub max_image_width: f64,
    pub max_image_height: f64,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            max_width: 500.0,
            line_height: 14.0,
            paragraph_spacing: 20.0,
            min_y_pos: 50.0,
            max_y_pos: 750.0,
            max_image_width: 500.0,
            max_image_height: 700.0,
        }
    }
}
