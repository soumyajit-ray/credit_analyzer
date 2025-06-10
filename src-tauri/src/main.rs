#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use tauri::command;
use chrono::NaiveDate;
use regex::Regex;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Transaction {
    date: String,
    description: String,
    amount: f64,
    category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnalysisResult {
    spending_categories: Vec<CategoryTotal>,
    top_merchants: Vec<MerchantTotal>,
    monthly_total: f64,
    insights: Vec<String>,
    transaction_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct CategoryTotal {
    category: String,
    total: f64,
    percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct MerchantTotal {
    merchant: String,
    total: f64,
    count: u32,
}

#[command]
async fn analyze_statement(file_path: String) -> Result<AnalysisResult, String> {
    println!("Analyzing file: {}", file_path);
    
    // Check if file exists
    if !std::path::Path::new(&file_path).exists() {
        return Err("File not found".to_string());
    }
    
    // Parse the file
    let transactions = match parse_file(&file_path) {
        Ok(txns) => txns,
        Err(e) => {
            println!("File parsing error: {}", e);
            // Return mock data if parsing fails, but mention it in insights
            return Ok(create_mock_analysis(&file_path, Some("Could not parse file - showing sample data".to_string())));
        }
    };
    
    if transactions.is_empty() {
        return Ok(create_mock_analysis(&file_path, Some("No transactions found in file".to_string())));
    }
    
    // Analyze real transactions
    let analysis = analyze_transactions(transactions, &file_path).await;
    Ok(analysis)
}

fn parse_file(file_path: &str) -> Result<Vec<Transaction>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    let mut transactions = Vec::new();
    
    if file_path.ends_with(".csv") {
        transactions = parse_csv(&content)?;
    } else if file_path.ends_with(".pdf") {
        // For PDF, you'd need more complex parsing
        return Err("PDF parsing not yet implemented".into());
    }
    
    println!("Parsed {} transactions", transactions.len());
    Ok(transactions)
}

fn parse_csv(content: &str) -> Result<Vec<Transaction>, Box<dyn std::error::Error>> {
    let mut transactions = Vec::new();
    let mut rdr = csv::Reader::from_reader(content.as_bytes());
    
    // Try to read headers
    let headers = rdr.headers()?.clone();
    println!("CSV Headers: {:?}", headers);
    
    for result in rdr.records() {
        let record = result?;
        
        if record.len() >= 3 {
            // Try to find date, description, and amount columns
            let date = record.get(0).unwrap_or("").to_string();
            let description = record.get(1).unwrap_or("").to_string();
            let amount_str = record.get(2).unwrap_or("0");
            
            // Clean and parse amount
            let amount = parse_amount(amount_str)?;
            
            // Skip header rows or invalid data
            if description.to_lowercase().contains("description") || 
               description.to_lowercase().contains("transaction") ||
               amount == 0.0 {
                continue;
            }
            
            transactions.push(Transaction {
                date,
                description,
                amount: amount.abs(), // Use absolute value for analysis
                category: None,
            });
        }
    }
    
    Ok(transactions)
}

fn parse_amount(amount_str: &str) -> Result<f64, Box<dyn std::error::Error>> {
    // Remove common currency symbols and formatting
    let cleaned = amount_str
        .replace("$", "")
        .replace(",", "")
        .replace("(", "-")
        .replace(")", "")
        .trim()
        .to_string();
    
    let amount = cleaned.parse::<f64>()?;
    Ok(amount)
}

async fn analyze_transactions(transactions: Vec<Transaction>, file_path: &str) -> AnalysisResult {
    let total_amount: f64 = transactions.iter().map(|t| t.amount).sum();
    
    // Categorize transactions
    let categorized = categorize_transactions(&transactions);
    let categories = calculate_categories(&categorized, total_amount);
    
    // Find top merchants
    let merchants = find_top_merchants(&transactions);
    
    // Generate insights
    let insights = generate_insights(&transactions, &categories, file_path);
    
    AnalysisResult {
        spending_categories: categories,
        top_merchants: merchants,
        monthly_total: total_amount,
        insights,
        transaction_count: transactions.len(),
    }
}

fn categorize_transactions(transactions: &[Transaction]) -> Vec<Transaction> {
    transactions.iter().map(|t| {
        let mut tx = t.clone();
        tx.category = Some(categorize_description(&t.description));
        tx
    }).collect()
}

fn categorize_description(description: &str) -> String {
    let desc_lower = description.to_lowercase();
    
    // Simple keyword-based categorization
    if desc_lower.contains("restaurant") || desc_lower.contains("food") || 
       desc_lower.contains("starbucks") || desc_lower.contains("mcdonald") ||
       desc_lower.contains("pizza") || desc_lower.contains("cafe") {
        "Food & Dining".to_string()
    } else if desc_lower.contains("gas") || desc_lower.contains("fuel") ||
              desc_lower.contains("shell") || desc_lower.contains("chevron") ||
              desc_lower.contains("exxon") || desc_lower.contains("uber") ||
              desc_lower.contains("lyft") {
        "Gas & Transportation".to_string()
    } else if desc_lower.contains("amazon") || desc_lower.contains("target") ||
              desc_lower.contains("walmart") || desc_lower.contains("store") {
        "Shopping".to_string()
    } else if desc_lower.contains("netflix") || desc_lower.contains("spotify") ||
              desc_lower.contains("movie") || desc_lower.contains("entertainment") {
        "Entertainment".to_string()
    } else if desc_lower.contains("pharmacy") || desc_lower.contains("medical") ||
              desc_lower.contains("doctor") || desc_lower.contains("health") {
        "Healthcare".to_string()
    } else {
        "Other".to_string()
    }
}

fn calculate_categories(transactions: &[Transaction], total: f64) -> Vec<CategoryTotal> {
    let mut category_totals: HashMap<String, f64> = HashMap::new();
    
    for tx in transactions {
        if let Some(category) = &tx.category {
            *category_totals.entry(category.clone()).or_insert(0.0) += tx.amount;
        }
    }
    
    let mut categories: Vec<CategoryTotal> = category_totals
        .into_iter()
        .map(|(category, amount)| CategoryTotal {
            category,
            total: amount,
            percentage: (amount / total) * 100.0,
        })
        .collect();
    
    categories.sort_by(|a, b| b.total.partial_cmp(&a.total).unwrap());
    categories
}

fn find_top_merchants(transactions: &[Transaction]) -> Vec<MerchantTotal> {
    let mut merchant_totals: HashMap<String, (f64, u32)> = HashMap::new();
    
    for tx in transactions {
        // Extract merchant name (first few words)
        let merchant = extract_merchant_name(&tx.description);
        let entry = merchant_totals.entry(merchant).or_insert((0.0, 0));
        entry.0 += tx.amount;
        entry.1 += 1;
    }
    
    let mut merchants: Vec<MerchantTotal> = merchant_totals
        .into_iter()
        .map(|(merchant, (total, count))| MerchantTotal {
            merchant,
            total,
            count,
        })
        .collect();
    
    merchants.sort_by(|a, b| b.total.partial_cmp(&a.total).unwrap());
    merchants.truncate(5); // Top 5 merchants
    merchants
}

fn extract_merchant_name(description: &str) -> String {
    // Simple merchant name extraction - take first 2-3 words
    let words: Vec<&str> = description.split_whitespace().take(2).collect();
    words.join(" ").to_uppercase()
}

fn generate_insights(transactions: &[Transaction], categories: &[CategoryTotal], file_path: &str) -> Vec<String> {
    let mut insights = Vec::new();
    
    insights.push(format!("Successfully analyzed {} transactions from {}", 
                         transactions.len(), 
                         file_path.split('/').last().unwrap_or(file_path)));
    
    if let Some(top_category) = categories.first() {
        insights.push(format!("Your largest spending category is {} at {:.1}% of total spending", 
                             top_category.category, top_category.percentage));
    }
    
    // Find frequent small transactions
    let small_transactions: Vec<_> = transactions.iter()
        .filter(|t| t.amount < 10.0)
        .collect();
    
    if small_transactions.len() > 5 {
        let small_total: f64 = small_transactions.iter().map(|t| t.amount).sum();
        insights.push(format!("You have {} small transactions (under $10) totaling ${:.2}", 
                             small_transactions.len(), small_total));
    }
    
    insights.push("Consider setting up spending alerts for your top categories".to_string());
    
    insights
}

fn create_mock_analysis(file_path: &str, additional_insight: Option<String>) -> AnalysisResult {
    let mut insights = vec![
        format!("File: {}", file_path.split('/').last().unwrap_or(file_path)),
    ];
    
    if let Some(insight) = additional_insight {
        insights.push(insight);
    }
    
    insights.extend(vec![
        "Showing sample data for demonstration".to_string(),
        "Upload a CSV with Date, Description, Amount columns for real analysis".to_string(),
    ]);
    
    AnalysisResult {
        spending_categories: vec![
            CategoryTotal {
                category: "Food & Dining".to_string(),
                total: 250.50,
                percentage: 35.2,
            },
            CategoryTotal {
                category: "Gas & Transportation".to_string(),
                total: 180.25,
                percentage: 25.3,
            },
        ],
        top_merchants: vec![
            MerchantTotal {
                merchant: "Sample Data".to_string(),
                total: 85.50,
                count: 12,
            },
        ],
        monthly_total: 712.45,
        insights,
        transaction_count: 0,
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![analyze_statement])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}