<?php
// INTENTIONAL VULNERABILITIES - DO NOT USE IN PRODUCTION
// Test fixture for security scanner validation

class VulnerablePHP {

    // PHP-001: SQL Injection
    public function searchUser($conn) {
        $id = $_GET['id'];
        $query = "SELECT * FROM users WHERE id = " . $id; // BAD: Concat
        mysql_query($query); // BAD: Deprecated and vulnerable
        mysqli_query($conn, "SELECT * FROM users WHERE name = " . $_POST['name']); // BAD
    }

    // PHP-002: Command Injection
    public function runCommand() {
        $cmd = $_GET['cmd'];
        exec("ls -la " . $cmd); // BAD: User input in exec
        shell_exec("cat " . $_POST['file']); // BAD
        system("grep " . $cmd . " /var/log/app.log"); // BAD
        passthru("echo " . $cmd); // BAD
        `ls -la $cmd`; // BAD: Backticks
    }

    // PHP-003: File Inclusion
    public function loadPage() {
        $page = $_GET['page'];
        include($page); // BAD: User-controlled include
        include_once($_POST['template']); // BAD
        require($page . ".php"); // BAD
        require_once($page); // BAD
    }

    // PHP-004: Insecure Unserialize
    public function deserialize() {
        $data = $_COOKIE['data'];
        $obj = unserialize($data); // BAD: Unserialize user input
        return unserialize($_GET['obj']); // BAD
    }

    // PHP-005: XSS
    public function displayContent() {
        echo $_GET['message']; // BAD: Direct echo of user input
        print $_POST['content']; // BAD: Direct print
        ?>
        <div><?= $_GET['data'] ?></div> <!-- BAD: Short echo tag -->
        <?php
    }

    // PHP-006: Path Traversal
    public function readFile() {
        $filename = $_GET['file'];
        file_get_contents($filename); // BAD: User input in file read
        fopen($_POST['path'], 'r'); // BAD
        readfile($_GET['download']); // BAD
    }

    // SAFE PATTERNS
    public function safeQuery($pdo) {
        $stmt = $pdo->prepare("SELECT * FROM users WHERE id = ?"); // GOOD
        $stmt->bindParam(1, $_GET['id'], PDO::PARAM_INT);
        $stmt->execute();
    }

    public function safeOutput() {
        echo htmlspecialchars($_GET['message'], ENT_QUOTES, 'UTF-8'); // GOOD
    }

    public function safeFile() {
        $filename = basename($_GET['file']); // GOOD: basename
        $realpath = realpath("/var/www/files/" . $filename); // GOOD: realpath
    }
}
?>
