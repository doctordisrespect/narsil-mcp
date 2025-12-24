package com.example.security;

// INTENTIONAL VULNERABILITIES - DO NOT USE IN PRODUCTION
// Test fixture for security scanner validation

import java.io.*;
import java.sql.*;
import javax.xml.parsers.*;

public class VulnerableJava {

    // JAVA-001: SQL Injection via string concatenation
    public void searchUser(String userId) throws SQLException {
        String query = "SELECT * FROM users WHERE id = " + userId; // BAD
        Statement stmt = connection.createStatement();
        stmt.executeQuery(query); // BAD: Executes with concatenated string
    }

    // JAVA-002: XXE Vulnerability
    public void parseXml(InputStream input) throws Exception {
        DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance(); // BAD: Default config
        DocumentBuilder builder = factory.newDocumentBuilder();
        builder.parse(input);
    }

    // JAVA-003: Insecure Deserialization
    public Object deserializeData(InputStream stream) throws Exception {
        ObjectInputStream ois = new ObjectInputStream(stream); // BAD
        return ois.readObject(); // BAD: Arbitrary object deserialization
    }

    // JAVA-004: Path Traversal
    public void readFile(HttpServletRequest request) throws IOException {
        String filename = request.getParameter("file");
        File file = new File("/data/" + filename); // BAD: User input in path
        Files.readAllBytes(Paths.get(request.getParameter("path"))); // BAD
    }

    // JAVA-005: LDAP Injection
    public void searchLdap(String username) throws NamingException {
        String filter = "(uid=" + username + ")"; // BAD: String concat
        ctx.search("ou=users,dc=example,dc=com", filter, null); // BAD
    }

    // SAFE PATTERNS (should not trigger)
    public void safeSearch(String userId) throws SQLException {
        PreparedStatement pstmt = connection.prepareStatement(
            "SELECT * FROM users WHERE id = ?");
        pstmt.setString(1, userId); // GOOD: Parameterized
        pstmt.executeQuery();
    }
}
