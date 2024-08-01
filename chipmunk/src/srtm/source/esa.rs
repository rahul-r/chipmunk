use std::fs;
use std::io::{self, Cursor, Read};
use std::path::PathBuf;
use tokio_util::bytes::Bytes;
use zip::read::ZipArchive;

const ENDPOINT: &str = "http://step.esa.int/auxdata/dem/SRTMGL1";

pub async fn fetch(hgt_name: &str) -> anyhow::Result<()> {
    let endpoint = ENDPOINT;
    let url = format!("{endpoint}/{hgt_name}.SRTMGL1.hgt.zip");

    log::info!("Downloading {url}");
    let response = reqwest::get(&url).await?;
    if response.status() != 200 {
        anyhow::bail!(
            "Error downloading {url}: status code: {}",
            response.status()
        );
    }

    let content = io::Cursor::new(response.bytes().await?);
    log::info!("Download complete, unzipping");
    unzip(content, &format!("{hgt_name}.hgt"))
}

fn unzip(zipped_data: Cursor<Bytes>, file_to_extract: &str) -> anyhow::Result<()> {
    let mut archive = ZipArchive::new(zipped_data)?;

    let cache_dir = PathBuf::from("/tmp/chipmunk-cache");
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let Some(hgt_file_path_in_zip) = file.enclosed_name() else {
            continue;
        };
        let Some(file_name_in_zip) = hgt_file_path_in_zip.file_name() else {
            continue;
        };
        if file_name_in_zip != file_to_extract {
            continue;
        }

        let extracted_file = cache_dir.join(file_name_in_zip);

        let mut outfile = fs::File::create(&extracted_file)?;
        io::copy(&mut file, &mut outfile)?;
        log::info!(
            "File extracted to \"{extracted_file:?}\" ({} bytes)",
            file.size()
        );

        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        return Ok(());
    }
    anyhow::bail!("File not found in archive");
}
