FROM ubuntu:24.04

RUN apt-get update && \
    apt-get install -y wget ca-certificates

RUN wget -qO /usr/local/bin/p4d https://cdist2.perforce.com/perforce/r25.2/bin.linux26x86_64/p4d
RUN chmod +x /usr/local/bin/p4d

VOLUME ["/opt/perforce/p4root"]

EXPOSE 1666
CMD ["p4d", "-r", "/opt/perforce/p4root"]