// Utility functions used across multiple modules

pub mod markdown;
pub use markdown::{markdown_to_enhanced_html, markdown_to_html_with_user_prefs, EnhancedMarkdownRenderer};

pub mod syntax;
pub use syntax::{SyntaxHighlighter, HighlightConfig, highlight_code, highlight_code_with_theme};

// Import the markdown to_html function from the external crate
use ::markdown::to_html;

// Enhanced function to add DaisyUI classes and basic code styling
pub fn add_daisyui_classes(html: &str) -> String {
    let mut styled_html = html.to_string();

    // Process code blocks with basic styling FIRST (this removes pre>code blocks)
    styled_html = process_code_blocks_basic(&styled_html);

    // Table styling
    styled_html = styled_html.replace("<table>", r#"<table class="table table-zebra w-full">"#);

    if styled_html.contains("<kbd>") {
        styled_html = styled_html.replace("<kbd>", "<kbd class=\"kbd kbd-sm\">");
    }

    // Links styling
    styled_html = styled_html.replace(
        "<a href=",
        r#"<a class="link link-primary hover:underline" href="#,
    );

    // List styling - be more careful to avoid conflicts
    if styled_html.contains("<ul>") && !styled_html.contains("checkbox") {
        // Only style non-task lists
        styled_html = styled_html.replace("<ul>", "<ul class=\"space-y-2\">");
    }

    // For ordered lists (ol)
    if styled_html.contains("<ol>") {
        styled_html =
            styled_html.replace("<ol>", "<ol class=\"list-decimal list-inside space-y-2\">");
    }

    // List items (li) styling
    if styled_html.contains("<li>") && !styled_html.contains("menu") {
        styled_html = styled_html.replace("<li>", "<li class=\"hover:bg-base-300 rounded\">");
    }

    // Blockquote styling
    styled_html = styled_html
        .replace("<blockquote>", r#"<blockquote class="border-l-4 border-primary pl-4 italic my-4 bg-base-100 p-4 rounded">"#);

    // Heading styling
    styled_html = styled_html
        .replace("<h1>", "<h1 class=\"text-5xl font-bold mb-4\">")
        .replace("<h2>", "<h2 class=\"text-4xl font-bold mb-3\">")
        .replace("<h3>", "<h3 class=\"text-3xl font-bold mb-2\">")
        .replace("<h4>", "<h4 class=\"text-2xl font-bold mb-2\">")
        .replace("<h5>", "<h5 class=\"text-xl font-bold mb-1\">")
        .replace("<h6>", "<h6 class=\"text-lg font-bold mb-1\">");

    // Task list styling (input checkboxes)
    styled_html = styled_html
        .replace(
            r#"<input type="checkbox" disabled="" checked="" />"#,
            r#"<input type="checkbox" class="checkbox checkbox-primary" checked disabled />"#,
        )
        .replace(
            r#"<input type="checkbox" disabled="" />"#,
            r#"<input type="checkbox" class="checkbox checkbox-primary" disabled />"#,
        );

    // Delete/Strikethrough styling
    styled_html = styled_html.replace("<del>", "<del class=\"line-through text-base-content/60\">");

    // Paragraph styling - add some spacing
    styled_html = styled_html.replace("<p>", "<p class=\"mb-4\">");

    styled_html
}

// Process code blocks and add basic DaisyUI formatting
fn process_code_blocks_basic(html: &str) -> String {
    let mut result = String::new();
    let mut pos = 0;
    let html_len = html.len();

    while pos < html_len {
        // Look for the start of a code block
        if let Some(start_pos) = html[pos..].find("<pre><code") {
            let full_start = pos + start_pos;

            // Add content before the code block
            result.push_str(&html[pos..full_start]);

            // Determine if it's a fenced code block with language
            if html[full_start..].starts_with("<pre><code class=\"language-") {
                // Find language and code content
                let lang_start = full_start + "<pre><code class=\"language-".len();
                if let Some(lang_end) = html[lang_start..].find("\">") {
                    let lang = &html[lang_start..lang_start + lang_end];
                    let code_start = lang_start + lang_end + 2; // +2 for "\">

                    if let Some(code_end) = html[code_start..].find("</code></pre>") {
                        let code_end_full = code_start + code_end;
                        let code_content = &html[code_start..code_end_full];

                        // Process this code block with proper HTML escaping
                        let clean_code =
                            html_escape::decode_html_entities(code_content).to_string();
                        let escaped_code = clean_code
                            .replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('>', "&gt;");
                        let formatted_code = escaped_code.replace('\n', "<br/>");

                        result.push_str(&format!(
                            r#"<div class="mockup-code">
                <pre data-prefix="$"><code class="language-{}">{}</code></pre>
            </div>"#,
                            lang, formatted_code
                        ));

                        pos = code_end_full + "</code></pre>".len();
                        continue;
                    }
                }
            } else {
                // Plain code block without language
                let code_start = full_start + "<pre><code>".len();

                if let Some(code_end) = html[code_start..].find("</code></pre>") {
                    let code_end_full = code_start + code_end;
                    let code_content = &html[code_start..code_end_full];

                    // Process this code block
                    let clean_code = html_escape::decode_html_entities(code_content).to_string();
                    let escaped_code = clean_code
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;");
                    let formatted_code = escaped_code.replace('\n', "<br/>");

                    result.push_str(&format!(
                        r#"<div class="mockup-code">
                <pre data-prefix="$"><code>{}</code></pre>
            </div>"#,
                        formatted_code
                    ));

                    pos = code_end_full + "</code></pre>".len();
                    continue;
                }
            }

            // If parsing failed, just add the original content
            pos = full_start + 1;
        } else {
            // No more code blocks, add remaining content
            result.push_str(&html[pos..]);
            break;
        }
    }

    result
}

// Helper function to convert markdown to HTML using the markdown crate with basic features only
pub fn markdown_to_html(markdown: &str) -> String {
    let html = to_html(markdown);
    add_daisyui_classes(&html)
}

// Enhanced markdown to HTML conversion with Streamdown-inspired features
pub fn markdown_to_html_enhanced(markdown: &str, use_enhanced: bool) -> String {
    if use_enhanced {
        markdown_to_enhanced_html(markdown)
    } else {
        markdown_to_html(markdown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utils_module_standalone() {
        let test_markdown = "# Test Header\n\nThis is a `test` with some **bold** text.\n\n| Col1 | Col2 |\n|------|------|\n| A    | B    |";

        let html = markdown_to_html(test_markdown);

        // Verify that markdown processing works from utils module
        assert!(html.contains("<h1"));
        assert!(html.contains("table table-zebra"));
        assert!(html.contains("Test Header"));

        println!("✅ Utils module markdown processing works correctly!");
    }

    #[test]
    fn test_enhanced_markdown_function() {
        let test_markdown = "# Test Header\n\nThis is a `test` with some **bold** text.\n\n```rust\nlet x = 42;\n```";

        // Test enhanced markdown
        let enhanced_html = markdown_to_html_enhanced(test_markdown, true);
        assert!(enhanced_html.contains("code-block-container"));
        assert!(enhanced_html.contains("Rust"));

        // Test basic markdown (backward compatibility)
        let basic_html = markdown_to_html_enhanced(test_markdown, false);
        assert!(basic_html.contains("<h1"));
        assert!(!basic_html.contains("code-block-container"));

        println!("✅ Enhanced markdown function works correctly!");
    }
}
