
    if let Some(notifications) = doc.find(Attr("id", "notifications")).next() {
        if let Some(form) = notifications.find(Name("form")).next() {
            if let Some(submit) = form.find(Name("input")).find(|n| n.attr("type") == Some("submit")) {
                if let Some(value) = submit.attr("value") {
                    // Ekstrak jumlah pesan dari nilai tombol submit
                    if let Some(count) = value.split_whitespace().nth(1) {
                        if let Ok(count_num) = count.parse::<usize>() {
                            unsafe {
                                INBOX_COUNT = count_num;
                            }
                        }
                    }
                    
                    // Menyimpan isi pesan sepenuhnya
                    unsafe {
                        INBOX_CONTENT = Some(value.to_string());
                    }
                }
            }
        }
    }