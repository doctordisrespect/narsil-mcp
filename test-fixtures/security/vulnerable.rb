# INTENTIONAL VULNERABILITIES - DO NOT USE IN PRODUCTION
# Test fixture for security scanner validation

class VulnerableController < ApplicationController

  # RUBY-001: SQL Injection via string interpolation
  def search_user
    query = "SELECT * FROM users WHERE name = '#{params[:name]}'"  # BAD
    User.find_by_sql("SELECT * FROM users WHERE id = #{params[:id]}")  # BAD
    User.where("email = '#{params[:email]}'")  # BAD
  end

  # RUBY-002: Command Injection
  def run_command
    system("ls -la #{params[:path]}")  # BAD: String interpolation in system
    result = `grep #{params[:term]} /var/log/app.log`  # BAD: Backticks
    exec("cat #{params[:file]}")  # BAD: exec with interpolation
  end

  # RUBY-003: Mass Assignment
  def update_user
    @user.update(params[:user])  # BAD: No strong params
    User.create(params[:user])  # BAD: Mass assignment
  end

  # RUBY-004: Open Redirect
  def redirect_user
    redirect_to params[:url]  # BAD: Open redirect
    redirect_to request.referer  # BAD: Can be manipulated
  end

  # RUBY-005: XSS via ERB
  def render_content
    @content = params[:content].html_safe  # BAD: html_safe on user input
    raw params[:data]  # BAD: raw with user input
  end

  private

  # SAFE PATTERNS
  def safe_query
    User.where(name: params[:name])  # GOOD: Parameterized
    User.where("email = ?", params[:email])  # GOOD: Placeholder
  end

  def safe_command
    system("ls", "-la", Shellwords.escape(params[:path]))  # GOOD: Escaped
  end

  def user_params
    params.require(:user).permit(:name, :email)  # GOOD: Strong params
  end
end
