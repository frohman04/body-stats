package xyz.clieb.bodygraph

import org.apache.commons.math3.stat.regression.SimpleRegression
import org.apache.poi.ss.usermodel.WorkbookFactory

import java.io.{File, IOException}
import java.nio.file.{Files, Path, StandardCopyOption}
import java.time.{LocalDate, ZoneId}

import scala.util.{Failure, Success}

import co.theasi.plotly._
import co.theasi.plotly.writer.{FileOptions, PlotFile}
import scopt.OptionParser
import xyz.clieb.bodygraph.Closable._
import xyz.clieb.bodygraph.Timed._

object Main {
  def main(args: Array[String]): Unit = {
    parser.parse(args, Options()) match {
      case Some(s) => new Main().run(s.path.get.toPath)
      case None =>
    }
  }

  case class Options(path: Option[File] = None)

  val parser = new OptionParser[Options]("body-graphs") {
    head("body-graphs", "0.1")

    arg[File]("<file>").action((x, c) =>
      c.copy(path = Some(x)))

    checkConfig(c =>
      if (c.path.isEmpty) {
        failure("Must specify the weight tracker file")
      } else if (!c.path.get.exists()) {
        failure(s"The input file does not exist: ${c.path.get.getAbsolutePath}")
      } else if (!c.path.get.isFile) {
        failure(s"The input file is not a file: ${c.path.get.getAbsolutePath}")
      } else {
        success
      })
  }
}

class Main {
  def run(path: Path): Unit = {
    val records = timed(s"Reading data from file: ${path}") { readFile(path) }
    timed("Validating read data") { validateFile(records) }
    val outFile = timed("Drawing graph") { drawWeightGraph(records) }
    println(outFile)
  }

  def readFile(path: Path): Seq[Record] = {
    val tmpPath = Files.createTempFile("body-data", "")
    Files.copy(
      path,
      tmpPath,
      StandardCopyOption.REPLACE_EXISTING,
      StandardCopyOption.COPY_ATTRIBUTES)

    val records = try {
      closable(WorkbookFactory.create(tmpPath.toFile)) { workbook =>
        val sheet = workbook.getSheetAt(0)
        (sheet.getFirstRowNum + 1 to sheet.getLastRowNum)
            .map { case rowNum: Int =>
              val row = sheet.getRow(rowNum)
              Record(
                row.getCell(0).getDateCellValue.toInstant.atZone(ZoneId.systemDefault()).toLocalDate,
                Option(row.getCell(2)).map(_.getNumericCellValue.toFloat),
                Option(row.getCell(3)).map(_.getNumericCellValue.toFloat),
                Option(row.getCell(4)).map(_.getNumericCellValue.toFloat),
                Option(row.getCell(5)).map(_.getNumericCellValue.toFloat),
                Option(row.getCell(6)).map(_.getNumericCellValue.toFloat),
                Option(row.getCell(7)).map(_.getNumericCellValue.toFloat),
              )
            }
      } match {
        case Success(value) => value
        case Failure(e) => throw e
      }
    } finally {
      Files.delete(tmpPath)
    }

    records
  }

  def validateFile(records: Seq[Record]): Unit = {
    val errors = (1 until records.size)
      .map(idx => {
        if (records(idx - 1).date.compareTo(records(idx).date) >= 0) {
          Some(s"Date for row ${idx - 1} (${records(idx - 1).date}) is the same or later " +
            s"than the date for row ${idx} (${records(idx).date}")
        } else {
          None
        }
      })
      .filter(_.isDefined)
    if (errors.nonEmpty) {
      throw new IOException(s"Found issues in data read from file: \n${errors.mkString("\n")}")
    }
  }

  def drawWeightGraph(records: Seq[Record]): PlotFile = {
    val relRecords = records.filter(_.weight.isDefined)

    val weights = timed("Calculate weight series") {
      weightSeries(relRecords)
    }
    val rollingAverage = timed("Calculate average series") {
      averageSeries(relRecords, 30)
    }
    val loessSR = timed("Calculate LOESS (SimpleLinear) curve series") {
      loessSimpleRegressionSeries(relRecords, 30)
    }

    val plot = Plot()
      .withScatter(
        weights.map(_._1),
        weights.map(_._2),
        ScatterOptions()
          .name("Weight")
          .mode(ScatterMode.Marker))
      .withScatter(
        rollingAverage.map(_._1),
        rollingAverage.map(_._2),
        ScatterOptions()
          .name("Rolling average"))
      .withScatter(
        loessSR.map(_._1),
        loessSR.map(_._2),
        ScatterOptions()
          .name("LOESS (SR)"))
      .xAxisOptions(AxisOptions().title("Date"))
      .yAxisOptions(AxisOptions().title("Weight (lbs)"))
    draw(plot, "weight", FileOptions(overwrite = true))
  }

  private def weightSeries(records: Seq[Record]): Seq[(String, Double)] =
    records.map(record => (record.date.toString, record.weight.get.toDouble))

  private def averageSeries(records: Seq[Record], numDays: Int): Seq[(String, Double)] = {
    records.map(record => {
      val lowerBound = record.date.minusDays(numDays / 2)
      val upperBound = record.date.plusDays((numDays - 1) / 2)

      val windowDays = records
          .filter(record => lowerBound.compareTo(record.date) <= 0 && record.date.compareTo(upperBound) <= 0)
          .map(record => record.weight.get)
      (record.date.toString, (windowDays.sum / windowDays.size).toDouble)
    })
  }

  private def loessSimpleRegressionSeries(records: Seq[Record], numDays: Int): Seq[(String, Double)] = {
    val baseDate = records.map(_.date).min
    records
        .map(record => {
          val lowerBound = record.date.minusDays(numDays / 2)
          val upperBound = record.date.plusDays((numDays - 1) / 2)

          val windowDays = records
              .filter(record => lowerBound.compareTo(record.date) <= 0 && record.date.compareTo(upperBound) <= 0)
              .map(record => (record.date.toEpochDay - baseDate.toEpochDay, record.weight.get))

          val regression = new SimpleRegression()
          windowDays.foreach(day => regression.addData(day._1, day._2))
          (
              record.date.toString,
              regression.predict(record.date.toEpochDay - baseDate.toEpochDay)
          )
        })
  }

  implicit def orderedLocalDate: Ordering[LocalDate] = new Ordering[LocalDate] {
    def compare(x: LocalDate, y: LocalDate): Int = x compareTo y
  }
}

case class Record(
    date: LocalDate,
    weight: Option[Float],
    fatWeight: Option[Float],
    pctFat: Option[Float],
    pctWater: Option[Float],
    pctBone: Option[Float],
    bmi: Option[Float])
