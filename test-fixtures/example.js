function validateEmail(email) {
    const trimmed = email.trim();
    const hasAt = trimmed.includes("@");
    const parts = trimmed.split("@");
    
    if (parts.length !== 2) {
        return { valid: false, error: "Invalid format" };
    }
    
    return { valid: true, domain: parts[1] };
}

function processItems(items, multiplier) {
    let total = 0;
    const results = [];
    
    for (const item of items) {
        const value = item.price * multiplier;
        total += value;
        results.push({ name: item.name, calculated: value });
    }
    
    return { total, results };
}

// Type error example
function buggyFunction(x) {
    if (typeof x === 'string') {
        return x.toUpperCase();
    }
    return x.toUpperCase();  // Bug: x is not string here
}
