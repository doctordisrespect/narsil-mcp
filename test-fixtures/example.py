def process_user(user_id, name):
    """Process a user and return their data."""
    result = {}
    result["id"] = user_id
    result["name"] = name.strip()
    result["name_length"] = len(name)
    
    if user_id > 0:
        result["valid"] = True
    else:
        result["valid"] = False
    
    return result

def calculate_total(items):
    """Calculate total price of items."""
    total = 0
    for item in items:
        price = item["price"]
        quantity = item["quantity"]
        total += price * quantity
    return total

def fetch_data(url):
    """Fetch data from a URL - has potential taint issues."""
    import urllib.request
    response = urllib.request.urlopen(url)  # taint source
    data = response.read()
    return data.decode("utf-8")
