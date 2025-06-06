#include <ctime>
#include <iostream>

#include "logger.hpp"

void Logger::Log(LogLevel level, const std::string& message) {
    std::string levelString = "";

    char timestamp[25];

    time_t now = std::time(0);
    strftime(timestamp, sizeof(timestamp), "[%Y-%m-%d %H:%M:%S] ",
             localtime(&now));

    std::cout << timestamp << levelString << ": " << message << std::endl;
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
