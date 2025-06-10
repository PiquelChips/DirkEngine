#include <fstream>

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

        Log& operator<<(const std::string& message);
        Log& operator<<(const char* message);

    private:
        std::ofstream file;
    };

public:
    Logger();

    Log Get(LogLevel level);

    static std::string GetLevelString(LogLevel level);

private:
    const std::string logPath = LOG_PATH;
};
