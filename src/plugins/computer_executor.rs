use crate::core::traits::{ExecutionEngine, AppError, Result};
use async_trait::async_trait;
use tokio::process::Command;
use image::{DynamicImage, Rgba, GenericImageView};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut};
use ab_glyph::{FontVec, PxScale};
use log::warn;
use std::fs;

pub struct ComputerExecutor;

impl ComputerExecutor {
    pub fn new() -> Self {
        Self
    }

    fn draw_grid(&self, img: &mut DynamicImage) -> Result<()> {
        let (width, height) = img.dimensions();
        let color = Rgba([255, 0, 0, 255]); // Red lines
        let rows = 10;
        let cols = 10;

        // Draw vertical lines
        for i in 0..=cols {
            let x = (i as f32 * (width as f32 / cols as f32)) as f32;
            draw_line_segment_mut(img, (x, 0.0), (x, height as f32), color);
        }

        // Draw horizontal lines
        for i in 0..=rows {
            let y = (i as f32 * (height as f32 / rows as f32)) as f32;
            draw_line_segment_mut(img, (0.0, y), (width as f32, y), color);
        }

        // Add labels (A0, B1, etc.)
        let font_data = fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf").ok();
        if let Some(data) = font_data {
            if let Ok(font) = FontVec::try_from_vec(data) {
                let scale = PxScale::from(24.0);
                for r in 0..rows {
                    for c in 0..cols {
                        let label = format!("{}{}", (b'A' + r as u8) as char, c);
                        let x = (c as f32 * (width as f32 / cols as f32)) + 5.0;
                        let y = (r as f32 * (height as f32 / rows as f32)) + 5.0;
                        draw_text_mut(img, Rgba([255, 255, 255, 255]), x as i32, y as i32, scale, &font, &label);
                    }
                }
            }
        }

        Ok(())
    }

    fn get_label_coordinates(&self, label: &str, width: u32, height: u32) -> Option<(u32, u32)> {
        if label.len() < 2 { return None; }
        let row_char = label.chars().next()?.to_ascii_uppercase();
        let col_char = label.chars().nth(1)?;

        let row = (row_char as u8).checked_sub(b'A')? as u32;
        let col = col_char.to_digit(10)? as u32;

        if row >= 10 || col >= 10 { return None; }

        let cell_w = width / 10;
        let cell_h = height / 10;

        let x = col * cell_w + cell_w / 2;
        let y = row * cell_h + cell_h / 2;

        Some((x, y))
    }
}

#[async_trait]
impl ExecutionEngine for ComputerExecutor {
    fn name(&self) -> &'static str {
        "ComputerExecutor"
    }

    async fn start_session(&mut self) -> Result<()> {
        let status = Command::new("xdotool")
            .arg("version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        
        if status.is_err() || !status.unwrap().success() {
            warn!("xdotool not found. Computer operation may fail.");
        }
        Ok(())
    }

    async fn execute(&mut self, code: &str, _language: &str) -> Result<String> {
        let mut result = String::new();
        let lines = code.lines();

        for line in lines {
            let line = line.trim();
            if line.is_empty() { continue; }

            if line == "screenshot" || line == "screenshot_annotated" {
                let path = "/tmp/lasada_screenshot.jpg";
                let output = Command::new("scrot").arg("-o").arg(path).output().await;
                if output.is_err() || !output.unwrap().status.success() {
                    let output = Command::new("gnome-screenshot").arg("-f").arg(path).output().await;
                    if output.is_err() || !output.unwrap().status.success() {
                        result.push_str("Error: Failed to take screenshot.\n");
                        continue;
                    }
                }

                if line == "screenshot_annotated" {
                    if let Ok(mut img) = image::open(path) {
                        self.draw_grid(&mut img).ok();
                        img.save(path).ok();
                    }
                }

                result.push_str(&format!("SCREENSHOT_SAVED: {}\n", path));
                continue;
            }

            if line.starts_with("click_label ") {
                let label = &line[12..];
                // Need dimensions
                let output = Command::new("xdotool").arg("getdisplaygeometry").output().await
                    .map_err(|e| AppError::ExecutionError(format!("xdotool error: {}", e)))?;
                let geom = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = geom.split_whitespace().collect();
                if parts.len() >= 2 {
                    let w: u32 = parts[0].parse().unwrap_or(1920);
                    let h: u32 = parts[1].parse().unwrap_or(1080);
                    if let Some((x, y)) = self.get_label_coordinates(label, w, h) {
                        Command::new("xdotool").arg("mousemove").arg(x.to_string()).arg(y.to_string()).arg("click").arg("1").status().await.ok();
                        result.push_str(&format!("Clicked grid cell {}\n", label));
                    } else {
                        result.push_str(&format!("Error: Invalid label {}\n", label));
                    }
                }
                continue;
            }

            if line.starts_with("type ") {
                let text = &line[5..];
                Command::new("xdotool").arg("type").arg(text).status().await.ok();
                result.push_str(&format!("Typed: {}\n", text));
                continue;
            }

            if line.starts_with("key ") {
                let key = &line[4..];
                Command::new("xdotool").arg("key").arg(key).status().await.ok();
                result.push_str(&format!("Pressed key: {}\n", key));
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            let output = Command::new("xdotool")
                .args(&parts)
                .output()
                .await
                .map_err(|e| AppError::ExecutionError(format!("xdotool error: {}", e)))?;

            if !output.status.success() {
                result.push_str(&format!("Error executing '{}': {}\n", line, String::from_utf8_lossy(&output.stderr)));
            } else {
                result.push_str(&format!("Executed: {}\n", line));
            }
        }

        Ok(result.trim().to_string())
    }

    async fn terminate(&mut self) -> Result<()> {
        Ok(())
    }
}
