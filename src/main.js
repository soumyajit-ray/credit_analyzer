import { invoke } from '@tauri-apps/api/core';

// Try to import dialog, but handle errors gracefully
let openDialog = null;
try {
    const dialogModule = await import('@tauri-apps/plugin-dialog');
    openDialog = dialogModule.open;
    console.log('Dialog module loaded successfully');
} catch (error) {
    console.error('Failed to load dialog module:', error);
}

async function analyzeStatement() {
    console.log('=== DIALOG DEBUG ===');
    console.log('Dialog function available:', !!openDialog);
    
    if (!openDialog) {
        alert('File dialog not available. Please use the manual file input below.');
        return;
    }
    
    try {
        console.log('Attempting to open dialog...');
        
        const selected = await openDialog({
            multiple: false,
            filters: [{
                name: 'Financial Files',
                extensions: ['csv', 'pdf', 'xlsx', 'xls']
            }]
        });
        
        console.log('Dialog result:', selected);
        
        if (!selected) {
            console.log('No file selected (user cancelled)');
            return;
        }
        
        await analyzeWithPath(selected);
        
    } catch (error) {
        console.error('Dialog error details:', error);
        console.error('Error type:', typeof error);
        console.error('Error message:', error.message);
        console.error('Error stack:', error.stack);
        alert('File dialog failed: ' + error.message + '\n\nPlease use the manual file input below.');
    }
}

async function analyzeFromInput() {
    const fileInput = document.getElementById('fileInput');
    
    if (!fileInput.files[0]) {
        alert('Please select a file first');
        return;
    }
    
    const fileName = fileInput.files[0].name;
    await analyzeWithPath(fileName);
}

async function analyzeWithPath(filePath) {
    const loadingDiv = document.getElementById('loading');
    const resultsDiv = document.getElementById('results');
    const analyzeBtn = document.getElementById('analyzeBtn');
    
    try {
        console.log('Starting analysis for:', filePath);
        
        // Show loading state
        loadingDiv.classList.remove('hidden');
        resultsDiv.classList.add('hidden');
        analyzeBtn.disabled = true;
        
        console.log('Calling backend...');
        
        // Call Rust backend
        const analysis = await invoke('analyze_statement', { 
            filePath: filePath 
        });
        
        console.log('Analysis completed:', analysis);
        
        // Display results
        displayResults(analysis);
        
    } catch (error) {
        console.error('Analysis failed:', error);
        alert('Analysis failed: ' + error.message);
    } finally {
        loadingDiv.classList.add('hidden');
        analyzeBtn.disabled = false;
    }
}

function displayResults(analysis) {
    const resultsDiv = document.getElementById('results');
    const categoriesDiv = document.getElementById('categories');
    const merchantsDiv = document.getElementById('merchants');
    const insightsDiv = document.getElementById('insights');
    
    // Display categories
    categoriesDiv.innerHTML = '<h3>Spending Categories</h3>';
    analysis.spending_categories.forEach(cat => {
        const item = document.createElement('div');
        item.className = 'category-item';
        item.innerHTML = `
            <span>${cat.category}</span>
            <span>$${cat.total.toFixed(2)} (${cat.percentage.toFixed(1)}%)</span>
        `;
        categoriesDiv.appendChild(item);
    });
    
    // Display top merchants
    merchantsDiv.innerHTML = '<h3>Top Merchants</h3>';
    analysis.top_merchants.forEach(merchant => {
        const item = document.createElement('div');
        item.className = 'merchant-item';
        item.innerHTML = `
            <span>${merchant.merchant}</span>
            <span>$${merchant.total.toFixed(2)} (${merchant.count} transactions)</span>
        `;
        merchantsDiv.appendChild(item);
    });
    
    // Display insights
    insightsDiv.innerHTML = '<div class="insights"><h3>Insights & Recommendations</h3><ul></ul></div>';
    const insightsList = insightsDiv.querySelector('ul');
    analysis.insights.forEach(insight => {
        const li = document.createElement('li');
        li.textContent = insight;
        insightsList.appendChild(li);
    });
    
    resultsDiv.classList.remove('hidden');
}

// Make functions available globally
window.analyzeStatement = analyzeStatement;
window.analyzeFromInput = analyzeFromInput;