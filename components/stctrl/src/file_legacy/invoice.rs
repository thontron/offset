use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use derive_more::*;

use app::ser_string::{from_base64, from_string, to_base64, to_string, SerStringError};
use app::PublicKey;

use app::invoice::InvoiceId;

use toml;

/// A helper structure for serialize and deserializing Invoice.
#[derive(Serialize, Deserialize)]
pub struct InvoiceFile {
    #[serde(serialize_with = "to_base64", deserialize_with = "from_base64")]
    pub invoice_id: InvoiceId,
    #[serde(serialize_with = "to_base64", deserialize_with = "from_base64")]
    pub dest_public_key: PublicKey,
    #[serde(serialize_with = "to_string", deserialize_with = "from_string")]
    pub dest_payment: u128,
}

impl From<SerStringError> for InvoiceFileError {
    fn from(_e: SerStringError) -> Self {
        InvoiceFileError::SerStringError
    }
}

/// Load Invoice from a file
pub fn load_invoice_from_file(path: &Path) -> Result<Invoice, InvoiceFileError> {
    let data = fs::read_to_string(&path)?;
    let invoice_file: InvoiceFile = toml::from_str(&data)?;

    Ok(Invoice {
        invoice_id: invoice_file.invoice_id,
        dest_public_key: invoice_file.dest_public_key,
        dest_payment: invoice_file.dest_payment,
    })
}

/// Store Invoice to file
pub fn store_invoice_to_file(invoice: &Invoice, path: &Path) -> Result<(), InvoiceFileError> {
    let Invoice {
        ref invoice_id,
        ref dest_public_key,
        dest_payment,
    } = invoice;

    let invoice_file = InvoiceFile {
        invoice_id: invoice_id.clone(),
        dest_public_key: dest_public_key.clone(),
        dest_payment: *dest_payment,
    };

    let data = toml::to_string(&invoice_file)?;

    let mut file = File::create(path)?;
    file.write_all(&data.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use app::invoice::{InvoiceId, INVOICE_ID_LEN};
    use app::PUBLIC_KEY_LEN;

    /*
    #[test]
    fn test_invoice_file_basic() {
        let invoice_file: InvoiceFile = toml::from_str(
            r#"
            invoice_id = 'invoice_id'
            dest_public_key = 'dest_public_key'
            dest_payment = '100'
        "#,
        )
        .unwrap();

        assert_eq!(invoice_file.invoice_id, "invoice_id");
        assert_eq!(invoice_file.dest_public_key, "dest_public_key");
        assert_eq!(invoice_file.dest_payment, "100");
    }
    */

    #[test]
    fn test_store_load_invoice() {
        // Create a temporary directory:
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("invoice_file");

        let invoice = Invoice {
            invoice_id: InvoiceId::from(&[0; INVOICE_ID_LEN]),
            dest_public_key: PublicKey::from(&[1; PUBLIC_KEY_LEN]),
            dest_payment: 100,
        };

        store_invoice_to_file(&invoice, &file_path).unwrap();
        let invoice2 = load_invoice_from_file(&file_path).unwrap();

        assert_eq!(invoice, invoice2);
    }
}