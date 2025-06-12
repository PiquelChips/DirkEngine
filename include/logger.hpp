#pragma once

#include <fstream>
#include <sstream>

enum LogLevel { DEBUG, INFO, WARNING, ERROR, FATAL };

/**
 *
 */
class Logger {

public:
    /**
     *
     */
    class Log {

    public:
        Log(LogLevel level, const std::string& filename);
        virtual ~Log();

        template <typename T> Log& operator<<(const T& value) {
            buffer << value;
            return *this;
        }

    private:
        std::ostringstream buffer;

        std::ofstream file;
    };

public:
    Logger();

    Log Get(LogLevel level);

    static std::string GetLevelString(LogLevel level);

private:
    const std::string logPath = LOG_PATH;
};
