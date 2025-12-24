// INTENTIONAL VULNERABILITIES - DO NOT USE IN PRODUCTION
// Test fixture for security scanner validation

// TS-001: Any type usage (bypasses type safety)
function processData(data: any): any { // BAD: any parameter
    const result: any = data.transform(); // BAD: any variable
    return result as any; // BAD: cast to any
}

interface UnsafeConfig {
    options: any; // BAD: any in interface
    handler: <any>(x: any) => any; // BAD: generic any
}

// TS-002: Non-null assertion operator (can cause runtime errors)
function getUserName(user?: User): string {
    return user!.name; // BAD: Non-null assertion on optional
}

function processItems(items?: Item[]): void {
    const first = items![0]; // BAD: Non-null assertion on optional array
    const length = items!.length; // BAD
}

class UserService {
    private user?: User;

    getName(): string {
        return this.user!.name; // BAD: Non-null assertion
    }

    getItems(): Item[] {
        return this.user!.items; // BAD: Non-null assertion
    }
}

// SAFE PATTERNS (should not trigger)
function safeProcess(data: unknown): string {
    if (typeof data === 'string') {
        return data; // GOOD: Type narrowing
    }
    return 'default';
}

function safeGetName(user?: User): string {
    return user?.name ?? 'Anonymous'; // GOOD: Optional chaining
}

function safeGetItems(items?: Item[]): Item | undefined {
    return items?.[0]; // GOOD: Optional chaining
}

// Types for reference
interface User {
    name: string;
    items: Item[];
}

interface Item {
    id: number;
    value: string;
}
