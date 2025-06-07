#include <ctime>
#include <iostream>
#include <sstream>

#include "logger.hpp"

Logger::Log::Log(LogLevel level, const std::string& filename) {

    file.open(filename);
    std::string levelString{GetLevelString(level)};

    char timestamp[25];

    time_t now = std::time(0);
    strftime(timestamp, sizeof(timestamp), "[%Y-%m-%d %H:%M:%S] ",
             localtime(&now));

    std::ostringstream out;
    out << timestamp << levelString << ": ";

    std::string outString{out.str()};

    std::cout << outString;

    if (file.is_open())
        file << outString;
}

Logger::Log::~Log() {
    std::cout << std::endl;

    if (file.is_open()) {
        file << std::endl;

        file.flush();
        file.close();
    }
}

Logger::Log& Logger::Log::operator<<(const std::string& message) {
    std::cout << message;

    if (file.is_open())
        file << message;

    return *this;
}

Logger::Log& Logger::Log::operator<<(const char* message) {
    std::cout << message;

    if (file.is_open())
        file << message;

    return *this;
}

std::string Logger::GetLevelString(LogLevel level) {
    switch (level) {
    case DEBUG:
        return "DEBUG";
    case INFO:
        return "INFO";
    case WARNING:
        return "WARNING";
    case ERROR:
        return "ERROR";
    case FATAL:
        return "FATAL";
    default:
        return "";
    }
}

Logger::Logger(const std::string& filename) : filename(filename) {
    if (filename == "")
        Get(INFO) << "No log file specified";
}

Logger::Log Logger::Get(LogLevel level) { return Log(level, filename); }
