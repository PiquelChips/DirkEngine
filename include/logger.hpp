#include <string>

enum LogLevel { DEBUG, INFO, WARNING, ERROR, FATAL };

class Logger {

public:
    void Log(LogLevel level, const std::string& message);

private:
    std::string GetLevelString(LogLevel level);
};
