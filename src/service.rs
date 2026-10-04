// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::error::Result;
use std::process::{Child, Command};

use crate::{config::Config, process_pdf_to_text::PdfBackend};

/// Starts the configured local service for a PDF backend.
///
/// The concrete command is deliberately kept in the configuration. This keeps
/// virtual environments, installation paths, and backend-specific arguments
/// outside the Rust implementation.
pub fn start_service(config: &Config, backend: PdfBackend) -> Result<Child> {
    let (binary, workdir, args) = match backend {
        PdfBackend::Grobid => (
            &config.grobid.server_binary,
            &config.grobid.server_workdir,
            &config.grobid.server_args,
        ),
        PdfBackend::Mineru => (
            &config.mineru.server_binary,
            &config.mineru.server_workdir,
            &config.mineru.server_args,
        ),
    };

    println!(
        "starting {} service: {} {}",
        backend,
        binary.display(),
        args.join(" ")
    );

    Ok(Command::new(binary)
        .current_dir(workdir)
        .args(args)
        .spawn()?)
}
