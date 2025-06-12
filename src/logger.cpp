#include <cstring>
#include <ctime>
#include <filesystem>
#include <iostream>
#include <sstream>
#include <string>

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
    std::string message = buffer.str();

    std::cout << message << std::endl;

    if (file.is_open()) {
        file << message << std::endl;

        // file.flush();
        file.close();
    }
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

Logger::Logger() {
    if (logPath == "") {
        Get(INFO) << "No log path specified";
        return;
    }

    std::filesystem::create_directory(std::filesystem::path{logPath});
}

Logger::Log Logger::Get(LogLevel level) {
    std::string filepath{""};

    if (logPath != "") {
        filepath = logPath + "/latest.log";
    }

    return Log(level, filepath);
}
