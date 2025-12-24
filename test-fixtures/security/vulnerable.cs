// INTENTIONAL VULNERABILITIES - DO NOT USE IN PRODUCTION
// Test fixture for security scanner validation

using System;
using System.Data.SqlClient;
using System.IO;
using System.Runtime.Serialization.Formatters.Binary;
using System.Web.Mvc;

namespace VulnerableCSharp
{
    public class UserController : Controller
    {
        // CSHARP-001: SQL Injection
        public void GetUser(string userId)
        {
            var query = "SELECT * FROM users WHERE id = " + userId; // BAD
            var cmd = new SqlCommand(query); // BAD
            cmd.ExecuteReader(); // BAD
        }

        // CSHARP-002: Insecure Deserialization
        public object DeserializeData(Stream stream)
        {
            var formatter = new BinaryFormatter(); // BAD: BinaryFormatter is insecure
            return formatter.Deserialize(stream); // BAD
        }

        // CSHARP-003: XSS via Razor
        public ActionResult DisplayContent(string content)
        {
            ViewBag.Content = Html.Raw(content); // BAD: Raw HTML
            var html = new HtmlString(content); // BAD
            Response.Write(content); // BAD
            return View();
        }

        // CSHARP-004: Path Traversal
        public void ReadFile()
        {
            var path = Path.Combine("C:\\Data", Request.QueryString["file"]); // BAD
            var content = File.ReadAllText(Request.QueryString["path"]); // BAD
        }

        // CSHARP-005: LDAP Injection
        public void SearchLdap(string username)
        {
            var filter = "(&(objectClass=user)(uid=" + username + "))"; // BAD
            var searcher = new DirectorySearcher();
            searcher.Filter = filter; // BAD
        }

        // SAFE PATTERNS
        public void SafeQuery(string userId)
        {
            var cmd = new SqlCommand("SELECT * FROM users WHERE id = @id");
            cmd.Parameters.AddWithValue("@id", userId); // GOOD: Parameterized
        }
    }
}
