// src/core/xml.rs
use std::path::PathBuf;
use tokio::fs;
use crate::error::AppError;
use quick_xml::writer::Writer;
use quick_xml::events::{Event, BytesStart, BytesText, BytesEnd};
use std::io::Cursor;
use log::warn; // 用于记录文件读取错误
use anyhow::anyhow; // 显式导入 anyhow

/// 生成符合题目中指定格式的 XML，包含所有选中文件。
/// - documents 根节点
/// - 每个文件一个 <document index="N"> 节点
///   - <source>保存文件路径
///   - <document_content>保存文件内容（UTF-8）
///
/// 此实现示例会一次性将所有文件内容读进来，并在内存中组装成字符串。
pub async fn generate_xml(paths: &[PathBuf]) -> Result<String, AppError> {
    let indent_char = b' ';
    let indent_size = 4;
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), indent_char, indent_size);

    let map_xml_err = |e: quick_xml::Error| AppError::General(anyhow!("XML write error: {:?}", e));

    writer.write_event(Event::Start(BytesStart::new("documents"))).map_err(map_xml_err)?;

    for (index, path) in paths.iter().enumerate() {
        let content = match fs::read_to_string(path).await {
            Ok(text) => text,
            Err(e) => {
                warn!("读取文件 {:?} 失败: {:?}, XML中将使用空内容", path, e);
                String::new()
            }
        };

        let mut doc_start = BytesStart::new("document");
        let index_str = (index + 1).to_string();
        doc_start.push_attribute(("index", index_str.as_str()));
        writer.write_event(Event::Start(doc_start)).map_err(map_xml_err)?;

        writer.write_event(Event::Start(BytesStart::new("source"))).map_err(map_xml_err)?;
        writer.write_event(Event::Text(BytesText::new(&*path.to_string_lossy()))).map_err(map_xml_err)?;
        writer.write_event(Event::End(BytesEnd::new("source"))).map_err(map_xml_err)?;

        writer.write_event(Event::Start(BytesStart::new("document_content"))).map_err(map_xml_err)?;

        if !content.is_empty() {
            // 写入初始换行符
            writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?;

            // --- 移除内容缩进，直接写入原始带换行符的内容 --- 
            // 确保原始 content 末尾有换行符，以便下一行写入 closing_indent_str
            let content_with_trailing_newline = if content.ends_with('\n') {
                content
            } else {
                let mut owned = content;
                owned.push('\n');
                owned
            };
            writer.write_event(Event::Text(BytesText::from_escaped(&content_with_trailing_newline))).map_err(map_xml_err)?;
            // -------------------------------------------------

            // 在结束标签前写入正确的缩进 (level 2)
            let closing_indent_level = 2;
            let closing_indent_str = std::iter::repeat(indent_char as char)
                               .take(indent_size * closing_indent_level)
                               .collect::<String>();
            writer.write_event(Event::Text(BytesText::from_escaped(&closing_indent_str))).map_err(map_xml_err)?;
        }

        writer.write_event(Event::End(BytesEnd::new("document_content"))).map_err(map_xml_err)?;

        writer.write_event(Event::End(BytesEnd::new("document"))).map_err(map_xml_err)?;
    }

    writer.write_event(Event::End(BytesEnd::new("documents"))).map_err(map_xml_err)?;

    let result_vec = writer.into_inner().into_inner();
    let xml_string = String::from_utf8(result_vec)
        .map_err(|e| AppError::General(anyhow!("XML非UTF8编码: {:?}", e)))?;
    
    Ok(xml_string)
} 