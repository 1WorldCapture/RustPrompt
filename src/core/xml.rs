// src/core/xml.rs
use std::path::PathBuf;
use tokio::fs;
use crate::error::AppError;
use quick_xml::writer::Writer;
use quick_xml::events::{Event, BytesStart, BytesText, BytesEnd};
use std::io::Cursor;
use log::warn; // 用于记录文件读取错误
use anyhow::anyhow; // 显式导入 anyhow

// NEW: 引入我们生成目录树的函数
use super::tree::generate_project_tree_string;

/// 生成符合题目中指定格式的 XML，包含所有选中文件。
/// - documents 根节点
/// - 每个文件一个 <document index="N"> 节点
///   - <source>保存文件路径
///   - <document_content>保存文件内容（UTF-8）
///
/// 此实现示例会一次性将所有文件内容读进来，并在内存中组装成字符串。
pub async fn generate_xml(paths: &[PathBuf]) -> Result<String, AppError> {
    // MOD: 创建不带缩进的 Writer
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    let map_xml_err = |e: quick_xml::Error| AppError::General(anyhow!("XML write error: {:?}", e));

    // MOD: 写入 <documents> 根标签（不带缩进）
    writer.write_event(Event::Start(BytesStart::new("documents")))
          .map_err(map_xml_err)?;

    // NEW: 在根标签内先插入一个换行
    writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?;

    // NEW: 第一个 <document>：动态项目目录树
    // 获取当前工作目录或你指定的根目录
    let current_dir = std::env::current_dir().map_err(|e| AppError::General(anyhow!("无法获取当前目录: {:?}", e)))?;
    let project_tree = generate_project_tree_string(current_dir.as_path())?;

    let mut doc_start_tree = BytesStart::new("document");
    doc_start_tree.push_attribute(("index", "1")); // 第一个固定为 1
    writer.write_event(Event::Start(doc_start_tree)).map_err(map_xml_err)?;
    writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?; // 换行

    // <source>project-tree-structure.txt</source>
    writer.write_event(Event::Start(BytesStart::new("source"))).map_err(map_xml_err)?;
    writer.write_event(Event::Text(BytesText::new("project-tree-structure.txt"))).map_err(map_xml_err)?;
    writer.write_event(Event::End(BytesEnd::new("source"))).map_err(map_xml_err)?;
    writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?; // 换行

    // <document_content> (项目目录树)
    writer.write_event(Event::Start(BytesStart::new("document_content"))).map_err(map_xml_err)?;
    writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?; // 换行
    writer.write_event(Event::Text(BytesText::from_escaped(&project_tree))).map_err(map_xml_err)?;
    // 确保树内容后有换行
    if !project_tree.ends_with('\n') {
        writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?;
    }
    writer.write_event(Event::End(BytesEnd::new("document_content"))).map_err(map_xml_err)?;
    writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?; // 换行

    writer.write_event(Event::End(BytesEnd::new("document"))).map_err(map_xml_err)?;
    writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?; // 换行

    // NEW: 接下来插入用户选中的文件，从 index="2" 开始
    for (index, path) in paths.iter().enumerate() {
        let content = match fs::read_to_string(path).await {
            Ok(text) => text,
            Err(e) => {
                warn!("读取文件 {:?} 失败: {:?}, XML中将使用空内容", path, e);
                String::new()
            }
        };

        // 因为第一个文档用了 index=1，所以后面文档要从 2 开始
        let doc_index = index + 2;
        let mut doc_start = BytesStart::new("document");
        doc_start.push_attribute(("index", doc_index.to_string().as_str()));

        writer.write_event(Event::Start(doc_start)).map_err(map_xml_err)?;
        writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?; // 换行

        // <source>...</source>
        writer.write_event(Event::Start(BytesStart::new("source"))).map_err(map_xml_err)?;
        writer.write_event(Event::Text(BytesText::new(&*path.to_string_lossy()))).map_err(map_xml_err)?;
        writer.write_event(Event::End(BytesEnd::new("source"))).map_err(map_xml_err)?;
        writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?; // 换行

        // <document_content>...</document_content>
        writer.write_event(Event::Start(BytesStart::new("document_content"))).map_err(map_xml_err)?;
        writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?; // 换行

        // 写入文件原内容 + 换行
        if !content.is_empty() {
            // 若文件本身不带结尾换行，可以手动加一个
            let content_with_newline = if content.ends_with('\n') {
                content
            } else {
                let mut tmp = content;
                tmp.push('\n');
                tmp
            };
            writer.write_event(Event::Text(BytesText::from_escaped(&content_with_newline)))
                  .map_err(map_xml_err)?;
        }

        writer.write_event(Event::End(BytesEnd::new("document_content"))).map_err(map_xml_err)?;
        writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?; // 换行

        writer.write_event(Event::End(BytesEnd::new("document"))).map_err(map_xml_err)?;
        writer.write_event(Event::Text(BytesText::from_escaped("\n"))).map_err(map_xml_err)?; // 换行
    }

    // MOD: 最后写入 </documents> (不带缩进)
    writer.write_event(Event::End(BytesEnd::new("documents"))).map_err(map_xml_err)?;

    // 将内存中的字节转为字符串
    let result_vec = writer.into_inner().into_inner();
    let xml_string = String::from_utf8(result_vec)
        .map_err(|e| AppError::General(anyhow!("XML非UTF8编码: {:?}", e)))?;

    Ok(xml_string)
} 