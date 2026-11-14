FROM php:8.2-cli-bookworm@sha256:ce09ca1d4b532edf9b0347b44a1968a80d9175addb581c450ba0c9a43fa0c5ea

RUN apt-get update \
    && apt-get install -y --no-install-recommends git libonig-dev libzip-dev libicu-dev \
        libpng-dev libjpeg62-turbo-dev libfreetype6-dev \
    && docker-php-ext-configure gd --with-freetype --with-jpeg \
    && docker-php-ext-install -j2 gd intl mbstring pdo_mysql zip \
    && rm -rf /var/lib/apt/lists/*

RUN git -c http.lowSpeedLimit=1024 -c http.lowSpeedTime=60 clone --depth 1 \
        --branch '6.5.14+240624' https://github.com/LimeSurvey/LimeSurvey.git /opt/limesurvey \
    && test "$(git -C /opt/limesurvey rev-parse HEAD)" = ac3c44744e2c3769b3941c41e50b68419423ce67 \
    && rm -rf /opt/limesurvey/.git

COPY tests/limesurvey-reference/config.php /opt/limesurvey/application/config/config.php
COPY tests/limesurvey-reference/ReferenceCommand.php /opt/limesurvey/application/commands/ReferenceCommand.php
COPY tests/limesurvey-reference/SettingsCases.php /opt/limesurvey/application/commands/SettingsCases.php
COPY tests/limesurvey-reference/FilterCases.php /opt/limesurvey/application/commands/FilterCases.php
COPY tests/limesurvey-reference/DefaultCases.php /opt/limesurvey/application/commands/DefaultCases.php
COPY tests/limesurvey-reference/ExecutionCases.php /opt/limesurvey/application/commands/ExecutionCases.php
COPY tools/limesurvey/RustSurveySettingsCommand.php /opt/limesurvey/application/commands/RustSurveySettingsCommand.php
COPY tools/limesurvey/RustSurveyRuntimeCommand.php /opt/limesurvey/application/commands/RustSurveyRuntimeCommand.php
WORKDIR /opt/limesurvey
ENV TZ=UTC
CMD ["php", "-v"]
